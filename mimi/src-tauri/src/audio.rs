//! Microphone capture. Audio stays in memory only and is released after each dictation.
//!
//! The microphone is opened only while recording, so Windows' "microphone in use"
//! indicator is off the rest of the time.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SizedSample, StreamConfig};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// A recording longer than this stops collecting audio, to protect memory.
const MAX_SECONDS: usize = 600;
/// Device names starting with this play a WAV file instead of opening a microphone.
/// Tests use it so they never need a real microphone.
pub const TEST_DEVICE_PREFIX: &str = "test-wav:";

#[derive(Debug, Clone, Serialize)]
pub struct MicInfo {
    pub name: String,
    pub is_default: bool,
}

/// Lists microphones without opening any of them.
pub fn list_microphones() -> Result<Vec<MicInfo>, String> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().as_ref().and_then(device_name);
    let devices = host.input_devices().map_err(|error| error.to_string())?;
    Ok(devices
        .filter_map(|device| device_name(&device))
        .map(|name| MicInfo {
            is_default: Some(&name) == default_name.as_ref(),
            name,
        })
        .collect())
}

fn device_name(device: &Device) -> Option<String> {
    device.description().ok().map(|description| description.name().to_string())
}

/// The audio collected so far, shared with the recording callback.
pub struct Capture {
    samples: Mutex<Vec<f32>>,
    level: AtomicU32,
    ready: AtomicBool,
    rate: u32,
    failure: Mutex<Option<String>>,
}

impl Capture {
    fn new(rate: u32) -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(rate as usize * 60)),
            level: AtomicU32::new(0),
            ready: AtomicBool::new(false),
            rate,
            failure: Mutex::new(None),
        }
    }

    fn push_mono(&self, mono: &[f32]) {
        if mono.is_empty() {
            return;
        }
        let rms = (mono.iter().map(|s| s * s).sum::<f32>() / mono.len() as f32).sqrt();
        self.level.store(rms.to_bits(), Ordering::Relaxed);
        let limit = self.rate as usize * MAX_SECONDS;
        let mut samples = self.samples.lock().unwrap();
        if samples.len() < limit {
            samples.extend_from_slice(mono);
        }
        drop(samples);
        self.ready.store(true, Ordering::Release);
    }

    /// Loudness of the most recent audio, from 0 upward.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// True once real audio has actually arrived from the device.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn take_samples(&self) -> Vec<f32> {
        std::mem::take(&mut *self.samples.lock().unwrap())
    }

    pub fn failure(&self) -> Option<String> {
        self.failure.lock().unwrap().clone()
    }

    fn fail(&self, message: String) {
        *self.failure.lock().unwrap() = Some(message);
    }
}

enum Active {
    Stream(#[allow(dead_code)] cpal::Stream),
    Test(Arc<AtomicBool>),
}

enum Command {
    Start(Option<String>, Sender<Result<Arc<Capture>, String>>),
    Stop(Sender<()>),
}

/// Owns the microphone on its own thread, because Windows audio streams must stay on one thread.
pub struct Microphone {
    commands: Sender<Command>,
}

impl Default for Microphone {
    fn default() -> Self {
        Self::new()
    }
}

impl Microphone {
    pub fn new() -> Self {
        let (commands, inbox) = channel::<Command>();
        thread::Builder::new()
            .name("mimi-audio".into())
            .spawn(move || {
                let mut active: Option<Active> = None;
                for command in inbox {
                    match command {
                        Command::Start(device, reply) => {
                            close(&mut active);
                            let result = open(device).map(|(capture, opened)| {
                                active = Some(opened);
                                capture
                            });
                            let _ = reply.send(result);
                        }
                        Command::Stop(reply) => {
                            close(&mut active);
                            let _ = reply.send(());
                        }
                    }
                }
            })
            .expect("audio thread failed to start");
        Self { commands }
    }

    pub fn start(&self, device: Option<String>) -> Result<Arc<Capture>, String> {
        let (reply, answer) = channel();
        self.commands.send(Command::Start(device, reply)).map_err(|e| e.to_string())?;
        answer.recv().map_err(|e| e.to_string())?
    }

    pub fn stop(&self) {
        let (reply, answer) = channel();
        if self.commands.send(Command::Stop(reply)).is_ok() {
            let _ = answer.recv();
        }
    }
}

fn close(active: &mut Option<Active>) {
    if let Some(Active::Test(stop)) = active {
        stop.store(true, Ordering::Relaxed);
    }
    *active = None;
}

fn open(device: Option<String>) -> Result<(Arc<Capture>, Active), String> {
    if let Some(path) = device.as_deref().and_then(|name| name.strip_prefix(TEST_DEVICE_PREFIX)) {
        return open_test_file(PathBuf::from(path));
    }
    let host = cpal::default_host();
    let chosen = match &device {
        Some(name) => host
            .input_devices()
            .map_err(|error| error.to_string())?
            .find(|candidate| device_name(candidate).as_ref() == Some(name))
            .ok_or_else(|| format!("The microphone \"{name}\" isn't connected."))?,
        None => host
            .default_input_device()
            .ok_or_else(|| "No microphone was found. Please connect one and try again.".to_string())?,
    };
    let supported = chosen.default_input_config().map_err(explain_open_error)?;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let capture = Arc::new(Capture::new(config.sample_rate));

    let stream = match format {
        SampleFormat::F32 => build::<f32>(&chosen, config, capture.clone()),
        SampleFormat::F64 => build::<f64>(&chosen, config, capture.clone()),
        SampleFormat::I8 => build::<i8>(&chosen, config, capture.clone()),
        SampleFormat::I16 => build::<i16>(&chosen, config, capture.clone()),
        SampleFormat::I32 => build::<i32>(&chosen, config, capture.clone()),
        SampleFormat::U8 => build::<u8>(&chosen, config, capture.clone()),
        SampleFormat::U16 => build::<u16>(&chosen, config, capture.clone()),
        SampleFormat::U32 => build::<u32>(&chosen, config, capture.clone()),
        other => return Err(format!("This microphone uses an unsupported audio format ({other}).")),
    }?;
    stream.play().map_err(explain_open_error)?;
    Ok((capture, Active::Stream(stream)))
}

#[cfg(target_os = "macos")]
fn explain_open_error(error: impl std::fmt::Display) -> String {
    format!(
        "Mimi couldn't open the microphone. Check that it's connected and that Mimi is allowed in \
         System Settings, Privacy & Security, Microphone. ({error})"
    )
}

#[cfg(not(target_os = "macos"))]
fn explain_open_error(error: impl std::fmt::Display) -> String {
    format!(
        "Mimi couldn't open the microphone. Check that it's connected and that Windows allows \
         microphone access in Settings, Privacy & security, Microphone. ({error})"
    )
}

fn build<T>(device: &Device, config: StreamConfig, capture: Arc<Capture>) -> Result<cpal::Stream, String>
where
    T: SizedSample + Send + 'static,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    let on_error = capture.clone();
    let mut mono: Vec<f32> = Vec::with_capacity(4096);
    device
        .build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _: &_| {
                mono.clear();
                mono.extend(data.chunks_exact(channels.max(1)).map(|frame| {
                    frame.iter().map(|sample| sample.to_sample::<f32>()).sum::<f32>() / channels.max(1) as f32
                }));
                capture.push_mono(&mono);
            },
            move |error| on_error.fail(format!("The microphone stopped working: {error}")),
            None,
        )
        .map_err(explain_open_error)
}

/// Plays a WAV file at real speed through the same path as a real microphone.
fn open_test_file(path: PathBuf) -> Result<(Arc<Capture>, Active), String> {
    let (samples, rate) = read_wav_mono(&path)?;
    let capture = Arc::new(Capture::new(rate));
    let stop = Arc::new(AtomicBool::new(false));
    let (feed, halt) = (capture.clone(), stop.clone());
    thread::spawn(move || {
        let block = (rate as usize / 50).max(1);
        for chunk in samples.chunks(block) {
            if halt.load(Ordering::Relaxed) {
                return;
            }
            feed.push_mono(chunk);
            thread::sleep(Duration::from_millis(20));
        }
    });
    Ok((capture, Active::Test(stop)))
}

/// Reads a WAV file and mixes it down to one channel.
pub fn read_wav_mono(path: &std::path::Path) -> Result<(Vec<f32>, u32), String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = reader.spec();
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader.samples::<i32>().map(|s| s.unwrap_or(0) as f32 / scale).collect()
        }
    };
    Ok((crate::clean::to_mono(&interleaved, spec.channels as usize), spec.sample_rate))
}
