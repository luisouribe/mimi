//! Prepares recorded audio for the speech model: 16 kHz, a healthy level, silence
//! trimmed, and long recordings split at natural pauses.
//! Nothing here touches the words themselves, and no audio is ever saved to disk.

use crate::vad::{FRAME, Vad};

pub const TARGET_RATE: u32 = 16_000;

const FRAME_SECONDS: f32 = FRAME as f32 / TARGET_RATE as f32;
const PEAK_TARGET: f32 = 0.8;
const MAX_GAIN: f32 = 24.0;
/// Below this, a recording is treated as digital silence.
const MIN_PEAK: f32 = 0.0005;

/// A frame this likely to be speech keeps the recording's edges from being cut.
const EDGE_PROBABILITY: f32 = 0.3;
/// A frame this likely to be speech proves someone spoke.
const SPEECH_PROBABILITY: f32 = 0.5;
const MIN_SPEECH_FRAMES: usize = 4;
const EDGE_MARGIN_SECONDS: f32 = 0.35;

/// Longer recordings slow down sharply, so they're processed in pieces of at most this length.
const MAX_CHUNK_SECONDS: f32 = 28.0;
const MIN_CHUNK_SECONDS: f32 = 12.0;

/// Audio ready for the speech model.
pub struct Prepared {
    pub chunks: Vec<Vec<f32>>,
    pub speech_seconds: f64,
}

/// Averages interleaved channels down to one.
pub fn to_mono(input: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return input.to_vec();
    }
    input
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Changes the sample rate with a windowed-sinc filter, which also removes
/// frequencies too high for the new rate so they cannot distort the voice.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    const ZEROS: usize = 16;
    const STEPS_PER_ZERO: usize = 512;

    let ratio = to as f64 / from as f64;
    let cutoff = ratio.min(1.0) * 0.95;
    let table: Vec<f32> = (0..=ZEROS * STEPS_PER_ZERO)
        .map(|index| {
            let x = index as f64 / STEPS_PER_ZERO as f64;
            let sinc = if x == 0.0 {
                1.0
            } else {
                (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
            };
            let window = 0.5 + 0.5 * (std::f64::consts::PI * x / ZEROS as f64).cos();
            (sinc * window) as f32
        })
        .collect();
    let lookup = |distance: f64| -> f32 {
        let position = distance.abs() * STEPS_PER_ZERO as f64;
        let low = position as usize;
        if low >= table.len() - 1 {
            return 0.0;
        }
        let mix = (position - low as f64) as f32;
        table[low] + (table[low + 1] - table[low]) * mix
    };

    let reach = ZEROS as f64 / cutoff;
    let length = (input.len() as f64 * ratio).round() as usize;
    let gain = cutoff as f32;
    let mut output = Vec::with_capacity(length);
    for index in 0..length {
        let center = index as f64 / ratio;
        let first = (center - reach).ceil().max(0.0) as usize;
        let last = ((center + reach).floor() as usize).min(input.len() - 1);
        let mut sum = 0.0f32;
        for (offset, sample) in input[first..=last].iter().enumerate() {
            let distance = (first + offset) as f64 - center;
            sum += sample * lookup(distance * cutoff);
        }
        output.push(sum * gain);
    }
    output
}

/// Raises quiet recordings to a healthy level. Loud ones are left untouched.
pub fn normalize(samples: &mut [f32]) {
    let peak = samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if peak <= f32::EPSILON || peak >= PEAK_TARGET {
        return;
    }
    let gain = (PEAK_TARGET / peak).min(MAX_GAIN);
    for sample in samples.iter_mut() {
        *sample *= gain;
    }
}

/// The frames to keep, from just before the first speech to just after the last.
/// `None` means nobody spoke.
pub fn speech_bounds(probabilities: &[f32]) -> Option<(usize, usize)> {
    let spoken = probabilities.iter().filter(|p| **p >= SPEECH_PROBABILITY).count();
    if spoken < MIN_SPEECH_FRAMES {
        return None;
    }
    let first = probabilities.iter().position(|p| *p >= EDGE_PROBABILITY)?;
    let last = probabilities.iter().rposition(|p| *p >= EDGE_PROBABILITY)?;
    let margin = (EDGE_MARGIN_SECONDS / FRAME_SECONDS).ceil() as usize;
    Some((first.saturating_sub(margin), (last + 1 + margin).min(probabilities.len())))
}

/// Where to cut long speech, as frame positions. Each cut lands in the longest pause
/// available, so a word is never split unless someone talks 28 seconds without pausing.
pub fn chunk_cuts(probabilities: &[f32], max_frames: usize, min_frames: usize) -> Vec<usize> {
    let mut cuts = Vec::new();
    let mut start = 0;
    while probabilities.len() - start > max_frames {
        let window_start = start + min_frames;
        let window_end = start + max_frames;

        let mut best_center = window_end;
        let mut best_length = 0;
        let mut run_start = None;
        for position in window_start..=window_end {
            let quiet = probabilities[position] < SPEECH_PROBABILITY;
            match (quiet, run_start) {
                (true, None) => run_start = Some(position),
                (false, Some(begin)) => {
                    if position - begin > best_length {
                        best_length = position - begin;
                        best_center = begin + (position - begin) / 2;
                    }
                    run_start = None;
                }
                _ => {}
            }
        }
        if let Some(begin) = run_start
            && window_end + 1 - begin > best_length
        {
            best_length = window_end + 1 - begin;
            best_center = begin + (window_end + 1 - begin) / 2;
        }
        if best_length == 0 {
            best_center = window_start
                + probabilities[window_start..=window_end]
                    .iter()
                    .enumerate()
                    .min_by(|a, b| a.1.total_cmp(b.1))
                    .map_or(0, |(offset, _)| offset);
        }
        cuts.push(best_center);
        start = best_center;
    }
    cuts
}

/// Runs the whole preparation. `Ok(None)` means no speech was heard.
pub fn prepare(mono: &[f32], rate: u32, vad: &mut Vad) -> Result<Option<Prepared>, String> {
    let mut audio = resample(mono, rate, TARGET_RATE);
    let peak = audio.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if peak < MIN_PEAK {
        return Ok(None);
    }
    normalize(&mut audio);

    let probabilities = vad.probabilities(&audio)?;
    let Some((first, last)) = speech_bounds(&probabilities) else {
        return Ok(None);
    };
    let end = (last * FRAME).min(audio.len());
    let speech = &audio[first * FRAME..end];
    let kept = &probabilities[first..last];

    let max_frames = (MAX_CHUNK_SECONDS / FRAME_SECONDS) as usize;
    let min_frames = (MIN_CHUNK_SECONDS / FRAME_SECONDS) as usize;
    let mut chunks = Vec::new();
    let mut from = 0;
    for cut in chunk_cuts(kept, max_frames, min_frames) {
        chunks.push(speech[from * FRAME..cut * FRAME].to_vec());
        from = cut;
    }
    chunks.push(speech[from * FRAME..].to_vec());

    Ok(Some(Prepared {
        speech_seconds: speech.len() as f64 / TARGET_RATE as f64,
        chunks,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frequency: f32, rate: u32, seconds: f32, amplitude: f32) -> Vec<f32> {
        (0..(rate as f32 * seconds) as usize)
            .map(|i| (2.0 * std::f32::consts::PI * frequency * i as f32 / rate as f32).sin() * amplitude)
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn stereo_becomes_mono_by_averaging() {
        assert_eq!(to_mono(&[1.0, 0.0, 0.5, 0.5], 2), vec![0.5, 0.5]);
        assert_eq!(to_mono(&[0.25, 0.75], 1), vec![0.25, 0.75]);
    }

    #[test]
    fn resampling_gives_the_right_length_at_common_rates() {
        for rate in [8_000, 16_000, 22_050, 44_100, 48_000, 96_000] {
            let input = vec![0.0; rate as usize];
            let output = resample(&input, rate, TARGET_RATE);
            assert_eq!(output.len(), TARGET_RATE as usize, "rate {rate}");
        }
    }

    #[test]
    fn resampling_keeps_voice_frequencies() {
        for rate in [44_100, 48_000] {
            let output = resample(&tone(1_000.0, rate, 1.0, 0.5), rate, TARGET_RATE);
            let middle = &output[2_000..14_000];
            let expected = 0.5 / 2f32.sqrt();
            assert!((rms(middle) - expected).abs() < 0.01, "rate {rate}: {}", rms(middle));
        }
    }

    #[test]
    fn resampling_removes_frequencies_that_would_distort() {
        let output = resample(&tone(15_000.0, 48_000, 1.0, 0.5), 48_000, TARGET_RATE);
        assert!(rms(&output[2_000..14_000]) < 0.005, "{}", rms(&output[2_000..14_000]));
    }

    #[test]
    fn quiet_audio_is_raised_and_loud_audio_is_left_alone() {
        let mut quiet = tone(300.0, TARGET_RATE, 0.5, 0.1);
        normalize(&mut quiet);
        let peak = quiet.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!((peak - PEAK_TARGET).abs() < 0.01, "{peak}");

        let mut loud = tone(300.0, TARGET_RATE, 0.5, 0.95);
        let before = loud.clone();
        normalize(&mut loud);
        assert_eq!(loud, before);
    }

    #[test]
    fn a_few_stray_frames_do_not_count_as_speech() {
        let mut probabilities = vec![0.0; 100];
        probabilities[50] = 0.9;
        probabilities[51] = 0.9;
        assert_eq!(speech_bounds(&probabilities), None);
        assert_eq!(speech_bounds(&[]), None);
    }

    #[test]
    fn edges_keep_a_margin_around_speech() {
        let mut probabilities = vec![0.0; 200];
        probabilities[100..110].fill(0.9);
        let margin = (EDGE_MARGIN_SECONDS / FRAME_SECONDS).ceil() as usize;
        assert_eq!(speech_bounds(&probabilities), Some((100 - margin, 110 + margin)));
    }

    #[test]
    fn a_faint_final_word_is_kept_even_with_low_confidence() {
        let mut probabilities = vec![0.0; 200];
        probabilities[20..60].fill(0.9);
        probabilities[150] = 0.35;
        let (_, last) = speech_bounds(&probabilities).unwrap();
        assert!(last > 150);
    }

    #[test]
    fn short_speech_is_never_split() {
        assert!(chunk_cuts(&vec![0.9; 100], 875, 375).is_empty());
    }

    #[test]
    fn long_speech_is_cut_at_the_longest_pause() {
        let mut probabilities = vec![0.9; 1_500];
        probabilities[500..504].fill(0.1);
        probabilities[700..720].fill(0.1);
        let cuts = chunk_cuts(&probabilities, 875, 375);
        assert_eq!(cuts[0], 710);
        assert!(cuts.iter().all(|cut| *cut < 1_500));
    }

    #[test]
    fn nonstop_speech_is_still_split_and_every_piece_fits() {
        let probabilities = vec![0.9; 4_000];
        let cuts = chunk_cuts(&probabilities, 875, 375);
        let mut edges = vec![0];
        edges.extend(&cuts);
        edges.push(4_000);
        assert!(edges.windows(2).all(|pair| pair[1] > pair[0] && pair[1] - pair[0] <= 875));
    }
}
