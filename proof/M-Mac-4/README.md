# M-Mac-4 proof: recording indicator and chimes on Mac

Built on 2026-11-09. **Confirmed by the owner on 2026-11-09** ("si me funciona", and "Ya pega" once pasting worked in this version).

## What changed

- `src-tauri/src/mac/indicator.rs` shows the indicator design signed off on Windows at the bottom center of the screen. It doesn't take focus, lets clicks pass through, and is set to appear on every desktop and above full-screen apps.
- `sounds.rs` plays the same two generated chimes through `NSSound`.
- `macOSPrivateApi` is on, for the indicator's see-through background. It's in the shared `tauri.conf.json` and the shared feature list, because Tauri's build checks only read those. It only acts on Mac.

## Unit tests

`unit-tests.txt`: 115 passed, 0 failed, 5 skipped, the same five skipped since `proof/M-Mac-1`.

## From the owner's test

| Check | Result |
|---|---|
| Indicator and chimes | Working, in the owner's words |
| Pasted into Google Chrome, last two dictations | Both pasted and saved in the history |
| Stop speaking to text | 120 and 131 ms |
| Shortcut to recording | 61 and 120 ms. The goal is under 100 ms |

## Worth knowing

- Two copies of Mimi with different builds caused another blocked paste: the permission went to the copy in Applications while the newer copy was running. Keep one copy, in Applications.
- Shortcut to recording has reached 120 to 124 ms in some uses since the chime and indicator were added, against 57 to 66 ms before. Only a few uses have been measured.

## Not checked yet

- The indicator above full-screen apps and on a second desktop.
- The Windows build. These changes were only compiled on the Mac.
