# M0 proof: foundations and speed test

Measured on 2026-11-03 on this PC: i5-11400H, 40 GB memory, RTX 3060 with 6 GB.

## App shell

| Measure | Goal | Result |
|---|---|---|
| Opens from its .exe | Yes | Yes, see `mimi-app-opened.png` |
| Time for the window to appear | - | 0.23 s |
| Processor use while idle | About 0% | 0.04% |
| Memory while idle, no speech model loaded | - | 164 MB |

## Speech models

Test material: 25 real recordings per language, 690 seconds in total. These are people reading aloud clearly, not everyday dictation. Accuracy ignores punctuation and capitals. Japanese is scored per character.

| Model | Language | Errors | Typical time for 10 s of speech | First-ever clip | Memory |
|---|---|---|---|---|---|
| Parakeet V3 | English | 2.4% | 1.06 s | 0.66 s | 908 MB |
| Parakeet V3 | Spanish | 2.1% | 1.20 s | 0.66 s | 908 MB |
| Whisper Turbo, graphics card | English | 3.9% | 0.72 s | 26.75 s | 630 MB, plus graphics memory |
| Whisper Turbo, graphics card | Spanish | 1.9% | 0.54 s | 26.75 s | 630 MB, plus graphics memory |
| Whisper Turbo, graphics card | Japanese | 6.4% | 1.26 s | 26.75 s | 630 MB, plus graphics memory |
| Whisper Small, graphics card | English | 7.5% | 0.75 s | 20.29 s | 405 MB, plus graphics memory |
| Whisper Small, graphics card | Spanish | 4.6% | 0.71 s | 20.29 s | 405 MB, plus graphics memory |
| Whisper Small, graphics card | Japanese | 13.4% | 0.92 s | 20.29 s | 405 MB, plus graphics memory |

The first-ever clip ran in English. It is one measurement per model, repeated in each row.

## Punctuation

Share of clips containing any punctuation.

| Model | English | Spanish |
|---|---|---|
| Parakeet V3 | 25 of 25 | 25 of 25 |
| Whisper Turbo | 16 of 25 | 10 of 25 |
| Whisper Small | 25 of 25 | 25 of 25 |

## Program size

| Item | Size |
|---|---|
| Blank app shell | 6.2 MB |
| Speech engine test program | 77 MB, about 23 MB compressed |
| `DirectML.dll`, beside the test program | 17.7 MB |

## Mimi's animation

Measured in the real app at 30 frames per second, before two later small changes: a softer listening glow and drawing her once at startup.

| State | Processor, share of whole PC | Graphics chip |
|---|---|---|
| Mimi's window active | 1.6% to 1.8% | About 5% |
| Another app active | 0.05% | 0% |
| Minimized | 0% | 0% |

Motion was checked in an isolated preview: across 13 frames, her hair tips shifted by about 1 pixel and her face by up to 2 pixels, with no errors. The animation was also checked by eye in the preview pane. The final build compiled but was not reopened for testing.

## Limits of these results

- The simulated weak PC, 4 logical processors, ran Parakeet as fast as the full PC: 0.95 s against 1.06 s in English. This does not prove a truly weak PC will be fast, because these are still this PC's fast processor cores.
- With only 25 clips per language, small differences are not meaningful, such as 1.9% against 2.1%.
- Times cover speech recognition only, not microphone capture or pasting.
- Parakeet offers word-level timing, but its output has not been tested yet.

Raw results are in the `.json` and `.stdout.txt` files in this folder. To repeat the speed test, run `tools/benchmark/run_all.ps1`.
