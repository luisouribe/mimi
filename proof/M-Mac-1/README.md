# M-Mac-1 proof: Mimi builds and dictates on a Mac

Built on 2026-11-09 on an Apple Silicon Mac with macOS 26.7.2. The app is at `~/code/mimi-build/release/bundle/macos/Mimi.app`.

**Status: signed off by the owner on 2026-11-09 ("Sí, funcionó")**, after dictating with their own voice in the real window. The tests below ran hidden, with audio files.

## What changed

- The Windows-only libraries now apply only on Windows.
- Autostart, the recording indicator and pasting each have a simple Mac version in `src-tauri/src/mac/`. Windows keeps its own files, unchanged.
- The global shortcut on Mac does nothing yet. The button in Mimi's window is how to dictate for now.
- Whisper uses Metal, the Mac's graphics engine, in place of Vulkan.
- Mimi's data goes in `~/Library/Application Support/Mimi`.
- The app asks for the microphone with a message in Spanish, from `src-tauri/Info.plist`.
- `tools/mac/env.sh` holds the Mac build settings.

## Unit tests

`unit-tests.txt`: 104 passed, 0 failed, 5 skipped.

Three of the skipped tests check each app's own volume slider, which only Windows has. The other two are helper tools that are skipped on Windows too.

## Hidden dictation test

`app-selftest.json` is the third of three full runs in the real app. All three finished with no errors. Each one used audio files instead of the microphone, and a temporary data folder.

| Model | Clip | Text Mimi produced | Speech recognition |
|---|---|---|---|
| Parakeet | Spanish `k01` | Este libro es muy bueno. | 83 to 127 ms |
| Parakeet | Spanish `f03` | Pues no sé qué decirte. | 116 to 135 ms |
| Parakeet | English `e02` | I think we should leave early. | 135 to 172 ms |
| Parakeet | English `e03` | I think that the plan is very very good. | 101 to 111 ms |
| Whisper Turbo on Metal | Spanish `k01` | Este libro es muy bueno. | 836 to 857 ms |
| Whisper Small on Metal | English `e02` | I think we should leave early. | 212 to 214 ms |

Fillers and stutters were removed as on Windows. In `e03` the speaker repeats "the the", and Parakeet wrote "that the".

The same runs also confirmed:

- Each dictation was saved to the history, and search and delete work.
- Starting twice, stopping without starting and a missing microphone are all refused with a clear message.
- All three models were found inside the app.
- Mimi's fonts and her drawing loaded in the screens.
- Mimi never came to the front. The app in front was checked every second during all three runs.

## Problems found

- **The microphone gave only silence.** In the owner's first test, three recordings opened the microphone and received no sound, and macOS never asked for permission. The app's hardened runtime needs the audio-input entitlement, now in `src-tauri/Entitlements.plist`. Confirmed by the owner with their own voice.
- **Closing after using Whisper crashed.** whisper.cpp's Metal cleanup aborted on exit. Mimi now leaves directly once everything is saved. Fixed and confirmed in all three runs.
- **One unexplained hang.** The very first run of the first build stopped responding after loading Whisper Turbo, and was stopped after 4 minutes. It didn't happen again in six later runs that used Whisper. If Whisper ever freezes on first use, this is where to look.
- **A compiler mismatch on this Mac.** Its SDK 27.0 is newer than its command line tools 26.6, so linking failed. `tools/mac/env.sh` picks the matching SDK.

## Not checked yet

- The Windows build. These changes are written so Windows behaves the same, but they were only compiled on the Mac.
