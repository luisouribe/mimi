//! Silero speech detection: says which moments of a recording contain a human voice.
//! It works on shape rather than loudness, so soft speech isn't mistaken for silence.
//! The small model (MIT license) is built into the app, so it can never go missing.

use ort::inputs;
use ort::session::Session;
use ort::value::Tensor;

const MODEL: &[u8] = include_bytes!("../assets/silero_vad.onnx");

/// Audio samples judged at a time, at 16 kHz: 32 milliseconds.
pub const FRAME: usize = 512;
const CONTEXT: usize = 64;
const STATE_SIZE: usize = 2 * 128;

pub struct Vad {
    session: Session,
}

impl Vad {
    pub fn new() -> Result<Self, String> {
        let session = Session::builder()
            .map_err(|error| error.to_string())?
            .with_intra_threads(1)
            .map_err(|error| error.to_string())?
            .commit_from_memory(MODEL)
            .map_err(|error| error.to_string())?;
        Ok(Self { session })
    }

    /// The chance of speech in each 32-millisecond frame, from 0 to 1.
    pub fn probabilities(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        let mut state = vec![0.0f32; STATE_SIZE];
        let mut context = vec![0.0f32; CONTEXT];
        let mut output = Vec::with_capacity(samples.len() / FRAME + 1);
        let mut input = vec![0.0f32; CONTEXT + FRAME];

        for frame in samples.chunks(FRAME) {
            input[..CONTEXT].copy_from_slice(&context);
            input[CONTEXT..CONTEXT + frame.len()].copy_from_slice(frame);
            input[CONTEXT + frame.len()..].fill(0.0);

            let audio = Tensor::from_array(([1usize, CONTEXT + FRAME], input.clone())).map_err(|e| e.to_string())?;
            let memory = Tensor::from_array(([2usize, 1, 128], state.clone())).map_err(|e| e.to_string())?;
            let rate = Tensor::from_array(([0usize; 0], vec![16_000i64])).map_err(|e| e.to_string())?;
            let results = self
                .session
                .run(inputs!["input" => audio, "state" => memory, "sr" => rate])
                .map_err(|e| e.to_string())?;

            let (_, probability) = results["output"].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
            output.push(probability[0]);
            let (_, next) = results["stateN"].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
            state.copy_from_slice(next);
            context.copy_from_slice(&input[FRAME..]);
        }
        Ok(output)
    }
}
