# Mimi for Mac: guide for Claude on the Mac

You're continuing work on Mimi on the owner's Mac. Mimi was built and tested on Windows in an earlier Claude session, which you can't see. This file is everything you need from it. Read `PLAN.md` next, then the `proof/` folders as needed.

## Who you work for

- **The owner is a director, not a programmer.** They write in Spanish, so answer in Spanish, plainly, with no jargon.
- **Proof at every step.** Each milestone ends with a proof folder (`proof/Mxx/README.md`) and the owner's sign-off before the next starts.
- **Show mock-ups before a build.** The owner likes to see design options as screenshots before you build anything.
- **Be cost-aware** with models and paid services. Ask before spending.
- **Never touch the desktop without asking.** Don't take focus, press keys, play sounds, lower volume or capture the screen without agreeing a moment with the owner first. Hidden self-tests are fine.
- **Installs are the owner's to run.** Don't install the finished app for them; give them the installer.

## What Mimi is

An offline dictation app with no AI rewriting.

1. You hold a shortcut and talk. A chime plays and other apps go quiet.
2. When you let go, the text is cleaned of fillers and stutters.
3. It's pasted into whatever app you were using, and also saved in a searchable history inside Mimi.

Everything runs on the computer, with no internet. Mimi is an anime mascot shown on the home screen.

The Windows version is finished and signed off: milestones M0 to M16, see `PLAN.md`.

| Part | Technology |
|---|---|
| App frame | Tauri 2, Rust edition 2024 |
| Screens | React 19, TypeScript, Vite |
| Speech | Parakeet V3 through ONNX Runtime (`ort` =2.0.0-rc.12 with `transcribe-rs` =0.3.12). Whisper Turbo and Small through whisper.cpp |
| Voice detection | Silero, built into the app (`src-tauri/assets/silero_vad.onnx`) |
| History and snippets | SQLite with full-text search |

Keep `ort` at `=2.0.0-rc.12`, because `transcribe-rs` 0.3.12 doesn't compile with later versions.

## Getting set up

1. Install the tools, with the owner's approval:
   - Xcode command line tools (`xcode-select --install`)
   - Rust (rustup)
   - Node 20 or newer
   - CMake, which Whisper's build needs
2. Get the speech models (about 1.4 GB). They aren't on GitHub because they're too big. Run `sh tools/mac/get_models.sh` from the project folder. It downloads them into `models/` and checks every file matches the Windows copies exactly.
3. Install the screens' packages: run `npm install` in `mimi/`.
4. Before any `cargo` or `tauri` command, run `. tools/mac/env.sh` from the project folder. To build the app, run `npx tauri build --bundles app` in `mimi/`.

**Progress:** dictating (M-Mac-1), global shortcuts on Option + Space (M-Mac-2), pasting (M-Mac-3), the indicator and chimes (M-Mac-4), stable signing and lowering the whole volume (M-Mac-5), and the installer and start at sign-in (M-Mac-6) work. See `proof/`. Keep one copy of the app, in Applications. To make the installer, run `sh tools/mac/build_installer.sh`, and let the owner install it.

**Signing:** builds use the owner's personal certificate "Mimi Local Signing", made by `tools/mac/create_signing_certificate.sh` and picked up by `tools/mac/env.sh`. It keeps Accessibility across builds, confirmed in M-Mac-5. If pasting ever stops with "copied" outcomes and macOS logs `Failed to match existing code requirement`, have the owner run `tccutil reset Accessibility com.mimi.dictation` and add `/Applications/Mimi.app` again. Don't change this permission for them. To update, the owner replaces the copy in Applications. The shortcuts are system hot keys, not the event tap suggested below, and need no permission.

## What's already the same on Mac

These parts have no Windows code. Their unit tests should pass on Mac as they are:

- the cleanup rules in `tidy.rs` and `clean.rs`
- snippets
- history and search
- settings
- the voice detection
- the shortcut decision logic in `hotkey.rs` (`HotkeyLogic`)
- the screens
- the chime sounds (`sounds.rs` builds them as sound waves)

Run `cargo test --lib` first, before changing anything, to see where you stand.

## What has to change for Mac

Each item is Windows-only today.

| Area | Windows today | What Mac needs |
|---|---|---|
| Building at all | `windows-sys` and `windows` crates are plain dependencies | Move them under `[target.'cfg(windows)'.dependencies]` and put each Windows module behind `#[cfg(windows)]`, with a Mac version beside it |
| Whisper on the graphics card | `whisper-vulkan` feature, Vulkan settings in `engine::prepare_graphics_environment` | whisper.cpp's Metal backend on Mac. Parakeet runs on the processor and should just work |
| Global shortcuts (`hotkey.rs`) | Low-level keyboard hook (`WH_KEYBOARD_LL`) | A Quartz event tap (`CGEventTap`), which needs the Input Monitoring or Accessibility permission. Keep `HotkeyLogic` as it is, and feed it Mac key codes |
| Default shortcut | Hold Ctrl + Space, tap Ctrl + Shift + Space | macOS uses Ctrl + Space to switch keyboard languages, so Mac needs a different default. Show the owner options first, such as Option + Space |
| Pasting (`paste.rs`) | Clipboard plus `SendInput` Ctrl + V. Checks the target app is still in front, and restores the clipboard | `NSPasteboard` plus a Cmd + V event, which needs Accessibility. The app in front comes from `NSWorkspace` |
| Recording indicator (`indicator.rs`) | Win32 styles: never takes focus, clicks pass through, on top | Tauri's own `set_always_on_top`, `set_ignore_cursor_events` and a non-focusable window, or an `NSWindow` level |
| Silencing other apps (`quiet.rs`) | Each app's own volume slider (WASAPI). The levels are saved first, so a crash can't leave apps silent | macOS has no public per-app volume. Ask the owner which they prefer: lowering the whole Mac's volume (the chime would need to play before it drops), or pausing media. Keep the crash-safe restore |
| Chimes (`sounds.rs`) | `PlaySoundW` from memory | Play the same generated WAV through `cpal` or `NSSound` |
| Start at sign-in (`autostart.rs`) | Registry Run key | A Login Item: Tauri's autostart plugin or `SMAppService` |
| Packaging | NSIS installer, Windows DLLs in `bundle-files/` | A `.app` or `.dmg`. Add `tauri.macos.conf.json` without `bundle-files/` |
| Permissions | None needed | The microphone (`NSMicrophoneUsageDescription`), Accessibility and Input Monitoring. Explain each one to the owner in plain words before they see the prompts |
| Unsigned app | Not an issue | The first open needs right-click > Open, or the owner allows it in Privacy & Security |

Bugs fixed on Windows that matter for Mac too:

- **Track modifier keys yourself.** Follow Ctrl, Alt and Shift from your own key events. Don't ask the system at the moment the main key arrives: on Windows that gave wrong answers while Mimi's own window was in front (M12).
- **Keep the screen's backup.** The screen also catches the shortcut when Mimi's window is in front. That's in `App.tsx`, and should carry over as it is.
- **Hide the main key's release.** Always hide the release of the shortcut's main key, or it presses the focused button (M11).

## Testing without touching the desktop

**Hidden self-test.** The app has a hidden self-test, set by environment variables:

- `MIMI_SELF_TEST_OUT`: where to write the JSON report
- `MIMI_SELF_TEST_CLIPS`: the clips, separated by `;`, such as `path|secs|model|lang`, `snippet|phrase|text`, `history-check|word`, `wait|secs` or `clear-check`
- `MIMI_DATA_DIR`: a temporary data folder, so the owner's real data isn't touched

In self-test mode, Mimi never chimes, silences apps or pastes. See `tools/m9/build_m9.ps1` for how it's run on Windows.

**Screen pictures without a window.** `tools/m5/serve_screens.py` serves the built screens with sample data on port 1421. Options such as `?theme=dark`, `?view=settings` and `?stage=listening` show different states. The Windows scripts take pictures with headless Edge; on Mac use headless Chrome or Safari.

**The diagnostics log.** `diagnostics.log` in Mimi's data folder records shortcut keys and recording outcomes, never words. Read it first when a shortcut problem is reported.

## Design decisions to keep

These are all in `PLAN.md` and in memory on the PC:

- the Stage design: cool grey, a coral band, angular shapes
- a light mode, and a plum dark mode (#2b2231), never black
- the neon poster: hollow glowing "mimi" in lowercase, a band with screen lines, and 耳 beside Mimi
- 耳 in the title bar
- a sun and moon switch above LANGUAGE
- Mimi always smiling

## Suggested first milestone on Mac

**M-Mac-1:** Mimi opens on the Mac, dictates with the button inside her window, and shows the text in her history. No global shortcut or pasting yet.

**Proof:** the app opening, a dictation in Spanish and in English, and the unit test results. Then add the shortcut, pasting, the indicator, the chimes and quieting, and the installer, each with proof and the owner's sign-off.
