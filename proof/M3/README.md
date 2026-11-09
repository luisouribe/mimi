# M3 proof: languages and model switching

Measured on 2026-11-04 on this PC, with no visible window, microphone, clipboard or key presses.

**Your voice check, 2026-11-04:** you dictated in Japanese with Whisper Turbo on the real app and reported "Funcionó bien". The first attempt failed to reach this build because the old copy was still open. When you approved, Mimi's saved settings were Whisper Turbo with Detect automatically, so this check was probably not with Japanese chosen. Remembering settings after quitting and reopening wasn't confirmed in the visible app, only in the hidden test.

## Models with the final build

25 clear reading recordings per language. Errors ignore punctuation and capitals, and Japanese is scored per character. "Time for 10 s" is scaled from each clip's length. Full results are in `results.md`.

| Model | Language | Errors | Time for 10 s | Clips punctuated |
|---|---|---|---|---|
| Parakeet V3 (default) | English | 2.8% | 0.69 s | 25 of 25 |
| Parakeet V3 (default) | Spanish | 2.4% | 0.79 s | 25 of 25 |
| Whisper Turbo | English | 4.5% | 0.48 s | 25 of 25 |
| Whisper Turbo | Spanish | 1.7% | 0.30 s | 25 of 25 |
| Whisper Turbo | Japanese | 6.0% | 0.80 s | 25 of 25 |
| Whisper Small | English | 7.3% | 0.33 s | 25 of 25 |
| Whisper Small | Spanish | 4.9% | 0.29 s | 25 of 25 |
| Whisper Small | Japanese | 13.4% | 0.54 s | 25 of 25 |

Whisper's times use the graphics card. Some of Whisper's English errors are only "Mr." against the reference's "mister". With 25 clips, small differences aren't meaningful. Accuracy for Parakeet here is 2.8% and 2.4%, against 2.4% and 1.7% in M1: same clips and same model, but this run feeds the 16 kHz originals instead of the 48 kHz versions.

## Punctuation hint

Whisper Turbo, clips containing any punctuation.

| Setting | English | Spanish | Japanese |
|---|---|---|---|
| Language chosen, no hint | 16 of 25 | 11 of 25 | 5 of 25 |
| Language chosen, with hint | 25 of 25 | 25 of 25 | 25 of 25 |
| Detect automatically (no hint possible) | 16 of 25 | 11 of 25 | 5 of 25 |

Those no-hint runs were measured before the optimized build. The hint didn't hurt accuracy: English moved from 4.3% to 4.5%, Spanish from 2.0% to 1.7%, and Japanese from 6.6% to 6.0%.

## Effect of the optimized build

Whisper Turbo with the graphics card, time for 10 s of speech.

| Language | Before | After |
|---|---|---|
| English | 0.81 s | 0.48 s |
| Spanish | 0.68 s | 0.30 s |
| Japanese | 1.60 s | 0.80 s |

## Without a graphics card

Measured from clips finished before a 7-minute limit, so neither model covered all 75 clips.

| Model | Clips finished | Typical wait per clip |
|---|---|---|
| Whisper Small | 47 | About 8 s, whatever the clip's length |
| Whisper Turbo | 9 | About 36 s |

Before the fix, Whisper Small took 50 s or more per clip. Turbo is too slow to be practical on a PC without a graphics card. Parakeet doesn't need a graphics card.

## The real app, hidden

| Check | Result |
|---|---|
| Switch to Whisper Turbo with Japanese | Ready, correct Japanese text in 0.47 s |
| Switch language only | Instant, no reload |
| Switch to Whisper Small, then back to Parakeet | Ready in 0.9 s and 1.9 s |
| Silence and loud noise under Whisper | No invented text |
| Opening with Whisper and Japanese already saved | Loaded in 1.3 s |
| Simulated graphics card crash | Next launch switched to the processor and remembered it |

The first use of Whisper on the graphics card took about 27 s, once. After that, the driver remembers the preparation and it takes 1 to 2 s.

## Screens

The Settings tab was checked in an isolated preview with simulated data. A stray title found there was removed. That fix and the final build got a code check, but the final build's screens weren't reviewed again.

## Not tested yet

- A real graphics card crash, because only its aftermath was simulated
- A PC without Vulkan, which couldn't open Mimi at all at the moment. This is on the M8 list
- Mixed Spanish and English on Whisper

## Noted

- App size is 84.4 MB plus `DirectML.dll` at 17.7 MB, so the 20 MB goal looks unrealistic.
- Whisper reports no word-by-word timing here, so M4's pause rule for spoken commands needs a different approach on Whisper.
- Whisper Turbo was faster than Parakeet on this PC and more accurate in Spanish. Parakeet stays the default because it was more accurate in English, needs no graphics card and detects the language itself.
