# M1 proof: speak and see text

Measured on 2026-11-04 on this PC. No test opened a microphone or a visible window, so the real microphone has not been tested yet. That is the voice check below.

## Accuracy and speed on real speech

25 recordings each in English and Spanish, run through Mimi's full audio path. Word errors ignore punctuation and capitals. "Time for 10 s" is scaled from each clip's length. Full results are in `results.md`.

| Situation | English errors | Spanish errors | Time for 10 s, English | Time for 10 s, Spanish |
|---|---|---|---|---|
| Microphone at 48 kHz stereo | 2.4% | 1.7% | 0.88 s | 0.96 s |
| Microphone at 44.1 kHz | 2.4% | 2.0% | 0.88 s | 1.06 s |
| Very quiet microphone, 5% volume | 2.6% | 1.9% | 0.95 s | 1.00 s |
| Two seconds of silence around speech | 2.8% | 1.7% | 0.98 s | 1.07 s |
| Background noise | 2.8% | 2.5% | 0.93 s | 1.04 s |

| Direct measure | Result |
|---|---|
| Real clips of 8 to 12 seconds, from stop to text | Median 0.95 s, worst 1.09 s |
| Long dictation, 206 s of speech | 25.5 s, previously 59 s, with 3.5% errors |
| Peak memory on the long dictation | 926 MB, previously 2,076 MB |
| Memory with the model loaded | About 850 MB |
| Model loading when the app opens | 2 to 3 s, in the background |

## The real app, hidden

The app ran from its screens through to the engine, using WAV files in place of a microphone. Details are in `app-self-test.json`.

| Check | Result |
|---|---|
| English clip, 5.7 s of speech | Correct text in 0.49 s |
| Spanish clip stopped at 8 s, 7.6 s of speech | Correct text in 0.73 s |
| Silence, faint noise, loud noise, electrical hum | No text, reported as no speech |
| Stopping when not recording | Refused with a clear message |
| Starting twice | Refused with a clear message |
| A microphone that isn't connected | Refused with a clear message |
| Live voice level sent to the screen | About 20 times per second |
| Word-by-word timing | Working, needed for spoken commands in M4 |

The layout was also checked with simulated data at 1280 x 720 in an isolated preview: nothing overlaps or overflows.

## Your voice check, 2026-11-04

You reported that it worked well. These are the numbers from your 8 recordings, which contain no words. They don't record which microphone was used.

| Speech heard | Stop to text | Microphone ready after |
|---|---|---|
| 6.0 s | 0.55 s | 779 ms |
| 4.2 s | 0.36 s | 680 ms |
| 9.2 s | 0.77 s | 675 ms |
| 3.1 s | 0.28 s | 679 ms |
| None | 0.05 s | 124 ms |
| 3.1 s | 0.28 s | 117 ms |
| None | 0.05 s | 117 ms |
| 3.8 s | 0.33 s | 134 ms |

The microphone took 120 to 780 ms to open, so a word spoken right at the start could be lost. Two of the 8 recordings were judged as containing no speech. At least one was the silence test, but the other wasn't identified.

## Problems found and fixed

- Quiet microphones lost their final words with loudness-based trimming. Silero speech detection replaced it.
- Long dictations slowed sharply and used 2 GB of memory. They are now split at natural pauses.
- A speculative fix, adding silence after speech, did not help and cost about 0.2 s, so it was removed.

## Known limits

- In `es/023`, the final word "tres" after a pause is missing in 3 of the 5 situations. Speech detection does keep that audio, so this is the speech model's own behavior. Your voice check includes a similar case.
- In noise, `es/003` heard "pobre de" instead of "pobre bestia". This is a recognition error caused by the noise, and the word's audio was kept.
- These recordings are clear reading, not natural dictation.
- Stopping at any moment is accepted, so a word spoken while the microphone is still opening can be lost. Real opening times are unknown until the voice check, especially on Bluetooth.
- Goal status: 10 s of speech takes just under the 1-second goal at the median, but some clips run slightly over.
- Memory stays at about 850 MB while Mimi is open. M8 will look at releasing the model when idle.
