# Mimi: build plan

Mimi is a personal, offline dictation app for Windows, with a Mac version later. Press a shortcut, speak, stop, and clean text is pasted into the active text box and saved in the app's history. Audio stays on the PC, and no AI rewrites the words.

## Decisions

| Topic | Decision |
|---|---|
| Default model | Parakeet V3 (Spanish, English and 23 other European languages) |
| Other languages | Change the model in Settings. Whisper covers Japanese and 100+ languages |
| Platforms | Windows first, Mac after the PC version is finished |
| When text appears | After you stop talking |
| Shortcut | Hold Ctrl + Space to talk, or tap Ctrl + Shift + Space to start and stop. Both can be changed in Settings |
| After dictating | Pastes automatically and is saved in a chat-style history with a copy button |
| Design | Proposal A "Stage", with Mimi's face on the dark recording indicator |
| Mascot | Mimi, always smiling, on the home page |
| Use | Personal |

## Technology

- **Tauri 2:** the frame that turns our code into a light desktop app.
- **Rust:** the fast engine for audio, speech and typing.
- **React, TypeScript, Tailwind and Motion:** the screens and animation.
- **transcribe-rs, ONNX Runtime and whisper.cpp:** the software that runs the speech models.
- **Silero VAD:** detects when you are actually speaking.
- **SQLite with full-text search:** the history and snippets database, stored on your PC.

## Milestones

Each milestone needs your sign-off before the next one starts.

| # | Build | Proof |
|---|---|---|
| M0 | Tools, blank app, model speed test | App opening and a results table |
| M1 | Speak and see text in a test window | Accuracy on real speech, timings, both microphones |
| M2 | Ctrl + Space, recording indicator, paste into any app, basic history | Text read back from Notepad, Chrome, WhatsApp and Word |
| M3 | Languages and model switching in Settings | Spanish, English and Japanese results |
| M4 | Remove fillers, collapse stutters, spoken commands | Before-and-after table with tricky cases |
| M5 | Build the chosen Stage design and animated Mimi into the real app, with a Settings switch for the animation | Screenshots and video |
| M6 | Snippets | Voice triggers and accidental-trigger checks |
| M7 | Full history search and reuse | Search speed with 10,000 entries |
| M8 | Installer, slow-PC testing, network check | Desktop icon, simulated weak-PC results, network log |

All milestones, M0 to M8, were signed off by 2026-11-05. Mimi is installed on the owner's PC.

After the plan, the owner asked for four extras on 2026-11-05:

| # | Extra | Status |
|---|---|---|
| M9 | Remove a hesitant "este", start with Windows, delete all dictations | Signed off by the owner on 2026-11-05, proof in `proof/M9` |
| M10 | A chime when listening starts and stops, and other apps go silent while you talk | Signed off by the owner on 2026-11-05, proof in `proof/M10` |
| M11 | Two changeable shortcuts (hold, and tap to start and stop), maximize, and each model's languages | Signed off by the owner on 2026-11-05, proof in `proof/M11` |
| M12 | Shortcuts inside Mimi's own window, plus a diagnostics log | Signed off by the owner on 2026-11-05 ("ya jala"), proof in `proof/M12` |
| M13 | Dark mode, first in dark grey | Replaced by M14 after the owner compared three palettes |
| M14 | Plum dark mode, 耳 beside Mimi in both modes, and a sun and moon switch above LANGUAGE | Signed off by the owner on 2026-11-05 ("Me gusta"), proof in `proof/M14` |
| M15 | 耳 in the title bar, lowercase "mimi" behind her, and a recording indicator in light or plum to match | Built 2026-11-05, followed directly by M16 |
| M16 | The neon poster in both modes: hollow glowing "mimi", a band with screen lines and neon edges, glowing 耳 | Signed off by the owner on 2026-11-05 ("me encantó"), proof in `proof/M16` |
| M17 | Spoken lists: counting out loud makes a numbered list, and "viñeta" or "bullet" makes bullets | Built 2026-11-06. The owner's first test made no lists, which led to M18. Proof in `proof/M17` |
| M18 | Lists as people really count: no pauses, "primero" followed by "dos, tres", and the first number mid-sentence | Built 2026-11-06. The owner's test made the list, then asked how to end one, which led to M19. Proof in `proof/M18` |
| M19 | Ending a list ("y ya", "eso es todo", "punto y aparte") and writing a number said twice only once | Built 2026-11-06. The owner's next test showed Parakeet writing lists as digits, which led to M20. Proof in `proof/M19` |
| M20 | Lists written as digits or a mix of digits and words, anywhere in a sentence, tested across 44 ways of writing one list | Signed off by the owner on 2026-11-09 after testing with their own voice, proof in `proof/M20` |
| Mac | A Mac version | Not started. The project goes to the Mac through a private GitHub repository. `MAC.md` is the guide for Claude on the Mac |

## Speed goals on this PC

Measured results are in `proof/M0/README.md`.

| Goal | Status after M0 |
|---|---|
| Shortcut to listening: under 0.1 seconds | Not measured until M2 |
| Stop speaking to text, after 10 seconds of speech: under 1 second | Median 0.95 seconds in M1, worst 1.09 seconds, before pasting is added |
| Idle processor use: about 0% | Met for the blank app: 0.04% |
| App download without models: under 20 MB | At risk: the app is 28.5 MB plus DirectML.dll at 17.7 MB, before compression |

## Build notes

- Do not take focus, press keys or capture the desktop during tests without agreeing a moment with you first. M2 will need that agreement, because it pastes into real apps.
- For the M8 installer: Mimi needs `directml.dll`, `vulkan-1.dll` and Microsoft's C++ runtime just to open. A PC missing `vulkan-1.dll` couldn't open Mimi at all, even for Parakeet, so this must be solved and tested.
- Build through `tools/benchmark/build_all.cmd`. Without its settings, Whisper's code is built unoptimized.
- Whisper Turbo needs a chosen language for reliable punctuation. Detect automatically leaves many clips without any.
- Spoken commands depend on the punctuation around them, because Parakeet's word timings show no gap at a pause and Whisper has none. Word timings are now unused, so remove them in M8 if that doesn't slow anything.
- After changing the app icon, delete `resource.lib` and `resource.rc` from the build folder and touch `build.rs`. A normal rebuild keeps the old icon inside `mimi.exe`.
- Mimi is installed at `%LOCALAPPDATA%\Mimi`, the same folder as its data. For a new version, rebuild the installer with `tools/m8/build_m8.ps1` and ask the owner to run it. Don't install from Claude's shell: Windows redirects that install into Claude's private storage. `tools/m8/install_and_watch.ps1` is only for hidden network checks of that private copy.
- Keep the web view's `additionalBrowserArgs` the same on both windows. They turn off the engine's background connections, which reached Microsoft before.
- Other apps are lowered through their own Volume mixer sliders, which Windows remembers. Mimi saves their levels in `other-apps-volume.json` first, and restores them when she next opens if she ever closes while listening. The hidden self test never plays chimes or lowers apps.
- Mimi tracks Ctrl, Alt and Shift from her own hook events. Don't go back to asking Windows (`GetAsyncKeyState`): its answer was wrong while Mimi's own window was in front, which broke the shortcuts there. Mimi's window also passes on any shortcut keys that reach it, as a backup.
- `diagnostics.log` in the data folder notes shortcut keys and recording outcomes, never words. Read it first when a shortcut problem is reported.
- Keep Mimi's pigtails to a slight float, with no wave along them, because more than that looks fake.

- Compile speech code with `C:\mimi-build` as the output folder, because deeper paths break the Windows 260-character limit.
- Keep `ort` at `2.0.0-rc.12`, because transcribe-rs 0.3.12 does not compile with rc.15.
- Warm up Whisper in the background when it is selected, because its first clip took 20 to 27 seconds.
