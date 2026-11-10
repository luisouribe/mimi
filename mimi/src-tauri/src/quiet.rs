//! Lowers every other app's sound while Mimi listens, then brings it back.
//!
//! It uses each app's own volume, the same sliders as Windows' Volume mixer, so Mimi's chimes
//! stay audible and the main volume is never touched. Windows remembers those sliders even
//! after an app closes, so the original levels are written to a small file first. If Mimi
//! ever stops without restoring them, she restores them the next time she opens.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time::Duration;

/// How loud other apps are while Mimi listens. Zero is silent.
pub const QUIET_LEVEL: f32 = 0.0;
const FADE_OUT: Duration = Duration::from_millis(160);
/// A Mac lowers its whole volume, so the stop chime waits for this and it needs to be quick.
const FADE_IN: Duration = Duration::from_millis(if cfg!(target_os = "macos") { 200 } else { 450 });
/// Lets the stop chime start before other sounds return. A Mac plays it after they're back.
const WAIT_BEFORE_RETURNING: Duration = Duration::from_millis(if cfg!(target_os = "macos") { 0 } else { 180 });
const FADE_STEPS: u32 = 10;
/// Volume sliders are stored as decimals, so equal levels can differ very slightly.
const SAME_LEVEL: f32 = 0.02;

enum Command {
    Lower(Option<Sender<()>>),
    LowerAfter(Duration),
    Restore { fade: bool, reply: Option<Sender<()>> },
}

#[derive(Clone)]
pub struct Quieter {
    commands: Sender<Command>,
}

/// Which apps to lower.
#[derive(Clone, Copy)]
enum Targets {
    /// Every app except Mimi.
    AllButThisApp,
    /// One app only, so tests never touch the person's own sounds.
    #[allow(dead_code)]
    OnlyThisApp,
}

impl Quieter {
    /// `memory` is the file where original levels wait until they're restored.
    pub fn start(memory: PathBuf) -> Self {
        Self::with_targets(memory, Targets::AllButThisApp)
    }

    fn with_targets(memory: PathBuf, targets: Targets) -> Self {
        let (commands, inbox) = channel::<Command>();
        thread::Builder::new()
            .name("mimi-quiet".into())
            .spawn(move || {
                let mut worker = Worker::new(memory, targets);
                worker.recover();
                for command in inbox {
                    match command {
                        Command::Lower(reply) => {
                            worker.lower();
                            reply.map(|reply| reply.send(()));
                        }
                        // Waiting here keeps the order: a restore sent meanwhile runs afterwards.
                        Command::LowerAfter(delay) => {
                            thread::sleep(delay);
                            worker.lower();
                        }
                        Command::Restore { fade, reply } => {
                            worker.restore(fade);
                            reply.map(|reply| reply.send(()));
                        }
                    }
                }
            })
            .expect("the volume thread failed to start");
        Self { commands }
    }

    /// Fades other apps down, without waiting.
    pub fn lower(&self) {
        let _ = self.commands.send(Command::Lower(None));
    }

    /// Fades down after a delay, without waiting, so a chime can finish first.
    pub fn lower_after(&self, delay: Duration) {
        let _ = self.commands.send(Command::LowerAfter(delay));
    }

    /// Fades back up and waits until it's done, so a chime can follow at full volume.
    pub fn restore_and_wait(&self) {
        let (reply, done) = channel();
        if self.commands.send(Command::Restore { fade: true, reply: Some(reply) }).is_ok() {
            let _ = done.recv_timeout(Duration::from_secs(3));
        }
    }

    /// Fades other apps back up after a short pause, without waiting.
    pub fn restore(&self) {
        let _ = self.commands.send(Command::Restore { fade: true, reply: None });
    }

    /// Puts every level back at once and waits, for when Mimi is closing.
    pub fn restore_now(&self) {
        let (reply, done) = channel();
        if self.commands.send(Command::Restore { fade: false, reply: Some(reply) }).is_ok() {
            let _ = done.recv_timeout(Duration::from_secs(2));
        }
    }
}

/// The levels to restore, by each app's Windows audio name. Only numbers and app paths, never sound.
fn read_memory(path: &PathBuf) -> HashMap<String, f32> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_memory(path: &PathBuf, levels: &HashMap<String, f32>) {
    if levels.is_empty() {
        let _ = std::fs::remove_file(path);
        return;
    }
    if let Some(folder) = path.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    if let Ok(text) = serde_json::to_string(levels) {
        let _ = std::fs::write(path, text);
    }
}

/// The level at each step of a fade.
fn fade_levels(from: f32, to: f32) -> Vec<f32> {
    (1..=FADE_STEPS).map(|step| from + (to - from) * step as f32 / FADE_STEPS as f32).collect()
}

fn same(a: f32, b: f32) -> bool {
    (a - b).abs() <= SAME_LEVEL
}

#[cfg(windows)]
use windows_audio::{Session, sessions};

#[cfg(windows)]
mod windows_audio {
    use super::Targets;
    use windows::Win32::Media::Audio::{
        DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume,
        MMDeviceEnumerator, eRender,
    };
    use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree};
    use windows::core::Interface;

    /// One app's sound on one speaker or headset.
    pub struct Session {
        pub name: String,
        volume: ISimpleAudioVolume,
    }

    impl Session {
        pub fn level(&self) -> Option<f32> {
            unsafe { self.volume.GetMasterVolume().ok() }
        }

        pub fn set_level(&self, level: f32) {
            unsafe {
                let _ = self.volume.SetMasterVolume(level.clamp(0.0, 1.0), std::ptr::null());
            }
        }
    }

    pub fn prepare_thread() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
    }

    /// Every app currently able to play sound, on every connected speaker or headset.
    pub fn sessions(targets: Targets) -> Vec<Session> {
        let this_app = std::process::id();
        let mut found = Vec::new();
        unsafe {
            let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
                return found;
            };
            let Ok(devices) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else { return found };
            for index in 0..devices.GetCount().unwrap_or(0) {
                let Ok(device) = devices.Item(index) else { continue };
                let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { continue };
                let Ok(list) = manager.GetSessionEnumerator() else { continue };
                for position in 0..list.GetCount().unwrap_or(0) {
                    let Ok(control) = list.GetSession(position) else { continue };
                    let Ok(control) = control.cast::<IAudioSessionControl2>() else { continue };
                    let process = control.GetProcessId().unwrap_or(0);
                    let wanted = match targets {
                        Targets::AllButThisApp => process != this_app,
                        Targets::OnlyThisApp => process == this_app,
                    };
                    if !wanted {
                        continue;
                    }
                    let Ok(volume) = control.cast::<ISimpleAudioVolume>() else { continue };
                    let Ok(text) = control.GetSessionIdentifier() else { continue };
                    let name = text.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(text.0 as *const _));
                    found.push(Session { name, volume });
                }
            }
        }
        found
    }
}

struct Lowered {
    session: Session,
    original: f32,
}

struct Worker {
    memory: PathBuf,
    targets: Targets,
    lowered: Vec<Lowered>,
    /// Levels from an earlier run that still need restoring.
    waiting: HashMap<String, f32>,
}

impl Worker {
    fn new(memory: PathBuf, targets: Targets) -> Self {
        #[cfg(windows)]
        windows_audio::prepare_thread();
        Self { memory, targets, lowered: Vec::new(), waiting: HashMap::new() }
    }

    fn remember(&self) {
        let mut levels = self.waiting.clone();
        for item in &self.lowered {
            levels.entry(item.session.name.clone()).or_insert(item.original);
        }
        write_memory(&self.memory, &levels);
    }

    /// Restores levels left lowered by an earlier run, for apps that are open now.
    fn recover(&mut self) {
        self.waiting.extend(read_memory(&self.memory));
        if self.waiting.is_empty() {
            return;
        }
        for session in sessions(self.targets) {
            if let Some(original) = self.waiting.get(&session.name).copied()
                && session.level().is_some_and(|level| same(level, QUIET_LEVEL))
            {
                session.set_level(original);
            }
        }
        // An app that's closed now gets its level back the next time it plays while Mimi listens.
        let open: Vec<String> = sessions(self.targets).into_iter().map(|session| session.name).collect();
        self.waiting.retain(|name, _| !open.contains(name));
        write_memory(&self.memory, &self.waiting);
    }

    fn lower(&mut self) {
        if !self.lowered.is_empty() {
            return;
        }
        for session in sessions(self.targets) {
            let Some(level) = session.level() else { continue };
            // An app left quiet by an earlier run goes back to where it really was.
            let original = self.waiting.remove(&session.name).filter(|_| same(level, QUIET_LEVEL)).unwrap_or(level);
            if same(level, QUIET_LEVEL) && same(original, QUIET_LEVEL) {
                continue;
            }
            self.lowered.push(Lowered { session, original });
        }
        // Saved before anything changes, so a crash mid-fade can still be undone.
        self.remember();
        let steps: Vec<Vec<f32>> = self.lowered.iter().map(|item| fade_levels(item.original, QUIET_LEVEL)).collect();
        for step in 0..FADE_STEPS as usize {
            for (item, levels) in self.lowered.iter().zip(&steps) {
                item.session.set_level(levels[step]);
            }
            thread::sleep(FADE_OUT / FADE_STEPS);
        }
    }

    fn restore(&mut self, fade: bool) {
        if self.lowered.is_empty() {
            return;
        }
        // If you moved an app's slider while Mimi listened, your choice is kept.
        let lowered = std::mem::take(&mut self.lowered);
        let mut returning: Vec<(Lowered, f32)> = lowered
            .into_iter()
            .filter(|item| item.session.level().is_some_and(|level| same(level, QUIET_LEVEL)))
            .map(|item| (item, QUIET_LEVEL))
            .collect();
        if fade {
            thread::sleep(WAIT_BEFORE_RETURNING);
            let steps: Vec<Vec<f32>> = returning.iter().map(|(item, _)| fade_levels(QUIET_LEVEL, item.original)).collect();
            for step in 0..FADE_STEPS as usize {
                for ((item, last), levels) in returning.iter_mut().zip(&steps) {
                    if item.session.level().is_some_and(|level| same(level, *last)) {
                        item.session.set_level(levels[step]);
                        *last = levels[step];
                    }
                }
                thread::sleep(FADE_IN / FADE_STEPS);
            }
        }
        for (item, last) in &returning {
            if item.session.level().is_some_and(|level| same(level, *last)) {
                item.session.set_level(item.original);
            }
        }
        self.remember();
    }
}

#[cfg(not(windows))]
use other_systems::{Session, sessions};

#[cfg(not(windows))]
#[path = "mac/quiet_volume.rs"]
mod other_systems;

#[cfg(test)]
mod tests {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::Mutex;

    /// The tests below share this test program's own volume slider, so they take turns.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn memory_file(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("mimi-quiet-{name}-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    /// Plays silence, so this test program gets its own slider in Windows' Volume mixer.
    /// Nothing is heard, and no other app is touched.
    fn silent_player() -> Option<cpal::Stream> {
        let device = cpal::default_host().default_output_device()?;
        let config = device.default_output_config().ok()?;
        if config.sample_format() != cpal::SampleFormat::F32 {
            return None;
        }
        let stream = device
            .build_output_stream(
                config.into(),
                |data: &mut [f32], _| data.fill(0.0),
                |_| {},
                None,
            )
            .ok()?;
        stream.play().ok()?;
        thread::sleep(Duration::from_millis(300));
        Some(stream)
    }

    fn own_level() -> Option<f32> {
        sessions(Targets::OnlyThisApp).first().and_then(Session::level)
    }

    fn lower_and_wait(quieter: &Quieter) {
        let (reply, done) = channel();
        quieter.commands.send(Command::Lower(Some(reply))).unwrap();
        done.recv().unwrap();
    }

    fn restore_and_wait(quieter: &Quieter) {
        let (reply, done) = channel();
        quieter.commands.send(Command::Restore { fade: true, reply: Some(reply) }).unwrap();
        done.recv().unwrap();
    }

    #[test]
    fn fades_move_evenly_and_end_on_the_target() {
        let down = fade_levels(0.8, 0.0);
        assert_eq!(down.len(), FADE_STEPS as usize);
        assert!(down.windows(2).all(|pair| pair[1] < pair[0]));
        assert_eq!(*down.last().unwrap(), 0.0);
        assert!((fade_levels(0.0, 0.6).last().unwrap() - 0.6).abs() < 1e-6);
    }

    #[test]
    #[cfg_attr(not(windows), ignore = "needs each app's own volume slider, which only Windows has")]
    fn an_app_is_lowered_then_restored_to_its_own_level() {
        let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let Some(_player) = silent_player() else {
            eprintln!("no speaker to test with");
            return;
        };
        for session in sessions(Targets::OnlyThisApp) {
            session.set_level(0.7);
        }
        let memory = memory_file("round-trip");
        let quieter = Quieter::with_targets(memory.clone(), Targets::OnlyThisApp);

        lower_and_wait(&quieter);
        assert!(same(own_level().unwrap(), QUIET_LEVEL));
        let saved = read_memory(&memory);
        assert_eq!(saved.len(), 1, "{saved:?}");
        assert!(same(*saved.values().next().unwrap(), 0.7));

        restore_and_wait(&quieter);
        assert!(same(own_level().unwrap(), 0.7), "{:?}", own_level());
        assert!(!memory.exists());
        for session in sessions(Targets::OnlyThisApp) {
            session.set_level(1.0);
        }
    }

    #[test]
    #[cfg_attr(not(windows), ignore = "needs each app's own volume slider, which only Windows has")]
    fn a_slider_moved_while_listening_is_left_alone() {
        let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let Some(_player) = silent_player() else { return };
        let memory = memory_file("moved");
        let quieter = Quieter::with_targets(memory.clone(), Targets::OnlyThisApp);
        lower_and_wait(&quieter);
        for session in sessions(Targets::OnlyThisApp) {
            session.set_level(0.4);
        }
        restore_and_wait(&quieter);
        assert!(same(own_level().unwrap(), 0.4));
        assert!(!memory.exists());
        for session in sessions(Targets::OnlyThisApp) {
            session.set_level(1.0);
        }
    }

    #[test]
    #[cfg_attr(not(windows), ignore = "needs each app's own volume slider, which only Windows has")]
    fn levels_left_low_by_a_crash_come_back_on_the_next_start() {
        let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let Some(_player) = silent_player() else { return };
        for session in sessions(Targets::OnlyThisApp) {
            session.set_level(0.9);
        }
        let memory = memory_file("crash");
        let first = Quieter::with_targets(memory.clone(), Targets::OnlyThisApp);
        lower_and_wait(&first);
        // Mimi "crashes": the first copy never restores anything.
        drop(first);
        assert!(same(own_level().unwrap(), QUIET_LEVEL));
        assert!(memory.exists());

        let second = Quieter::with_targets(memory.clone(), Targets::OnlyThisApp);
        // Opening alone restores it. Sending a command just waits until opening is done.
        restore_and_wait(&second);
        assert!(same(own_level().unwrap(), 0.9), "{:?}", own_level());
        assert!(!memory.exists());
        for session in sessions(Targets::OnlyThisApp) {
            session.set_level(1.0);
        }
    }
}
