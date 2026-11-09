//! Mimi's two short chimes: rising when she starts listening, falling when she stops.
//! They're made here as sound waves, so no audio file has to ship with the app.

use std::io::Cursor;
use std::sync::OnceLock;

const RATE: u32 = 44_100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chime {
    /// "Tu-rún" going up: Mimi is listening.
    Start,
    /// "Tu-rún" going down: Mimi stopped listening.
    Stop,
}

/// Two notes a fourth apart. The second starts as the first fades, like a soft marimba.
const LOW: f32 = 659.25; // E5
const HIGH: f32 = 880.0; // A5
const FIRST_NOTE: f32 = 0.085;
const SECOND_NOTE: f32 = 0.24;
const LOUDNESS: f32 = 0.32;

fn notes(chime: Chime) -> [f32; 2] {
    match chime {
        Chime::Start => [LOW, HIGH],
        Chime::Stop => [HIGH, LOW],
    }
}

/// One plucked note: a quick rise, then a smooth fade, with a quiet overtone for warmth.
fn note_into(samples: &mut [f32], start: usize, frequency: f32, seconds: f32) {
    let length = (seconds * RATE as f32) as usize;
    let attack = (0.004 * RATE as f32) as usize;
    for i in 0..length.min(samples.len().saturating_sub(start)) {
        let t = i as f32 / RATE as f32;
        let rise = (i as f32 / attack as f32).min(1.0);
        let fade = (-t * 5.0 / seconds).exp() * (1.0 - i as f32 / length as f32).min(1.0);
        let phase = std::f32::consts::TAU * frequency * t;
        let tone = phase.sin() + 0.18 * (2.0 * phase).sin() + 0.05 * (3.0 * phase).sin();
        samples[start + i] += tone * rise * fade;
    }
}

/// The chime as raw sound, from -1 to 1.
pub fn samples(chime: Chime) -> Vec<f32> {
    let [first, second] = notes(chime);
    let second_start = (FIRST_NOTE * RATE as f32) as usize;
    let total = second_start + (SECOND_NOTE * RATE as f32) as usize;
    let mut samples = vec![0.0; total];
    note_into(&mut samples, 0, first, FIRST_NOTE + 0.05);
    note_into(&mut samples, second_start, second, SECOND_NOTE);
    let peak = samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    samples.iter().map(|sample| sample / peak * LOUDNESS).collect()
}

/// The chime as a complete WAV file in memory.
pub fn wav(chime: Chime) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut file = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut file, spec).expect("a WAV header always fits in memory");
    for sample in samples(chime) {
        writer.write_sample((sample * i16::MAX as f32) as i16).expect("writing to memory can't fail");
    }
    writer.finalize().expect("writing to memory can't fail");
    file.into_inner()
}

/// Plays the chime without waiting. Windows keeps reading the sound from memory while it plays,
/// so both chimes are made once and kept for as long as Mimi runs.
pub fn play(chime: Chime) {
    static START: OnceLock<Vec<u8>> = OnceLock::new();
    static STOP: OnceLock<Vec<u8>> = OnceLock::new();
    let sound = match chime {
        Chime::Start => START.get_or_init(|| wav(Chime::Start)),
        Chime::Stop => STOP.get_or_init(|| wav(Chime::Stop)),
    };
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
        use windows::core::PCWSTR;
        let _ = PlaySoundW(PCWSTR(sound.as_ptr().cast()), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_chime_is_a_short_valid_sound() {
        for chime in [Chime::Start, Chime::Stop] {
            let bytes = wav(chime);
            let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
            let seconds = reader.duration() as f32 / reader.spec().sample_rate as f32;
            assert!((0.25..0.4).contains(&seconds), "{chime:?} lasts {seconds} seconds");
            let loudest = reader.into_samples::<i16>().map(|s| s.unwrap().unsigned_abs()).max().unwrap();
            assert!(loudest > 8_000 && loudest < 12_000, "{chime:?} peak {loudest}");
        }
    }

    #[test]
    fn start_goes_up_and_stop_goes_down() {
        assert!(notes(Chime::Start)[0] < notes(Chime::Start)[1]);
        assert!(notes(Chime::Stop)[0] > notes(Chime::Stop)[1]);
        // Both end almost silent, so there's no click.
        for chime in [Chime::Start, Chime::Stop] {
            let samples = samples(chime);
            assert!(samples.last().unwrap().abs() < 0.01);
        }
    }
}

/// Saves both chimes as WAV files, so they can be heard and mixed into test recordings.
/// Run with `cargo test --lib -- --ignored save_chimes` and `MIMI_CHIME_OUT` set to a folder.
#[cfg(test)]
#[test]
#[ignore]
fn save_chimes() {
    let folder = std::path::PathBuf::from(std::env::var_os("MIMI_CHIME_OUT").expect("set MIMI_CHIME_OUT"));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("chime-start.wav"), wav(Chime::Start)).unwrap();
    std::fs::write(folder.join("chime-stop.wav"), wav(Chime::Stop)).unwrap();
}
