# M8 proof: installer, weak PCs and network

Checked on 2026-11-05 on this PC, first hidden, then by installing Mimi on your PC with your agreement.

## Installer

| Item | Result |
|---|---|
| File | `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe` |
| Size | 1.44 GB, because it includes all three speech models |
| Needs administrator rights | No, it installs for your user only |
| Installed on your PC | Yes, by you on 2026-11-05. You confirmed it works |
| Internet needed to install | No on this PC. A PC without Microsoft's WebView2 would download it, and Windows 11 includes it |

**Your data on uninstall:** the uninstaller only removes the files it installed, plus folders left empty, so history, snippets and settings stay. This comes from reading the installer's script, not from running an uninstall.

## The first install went to the wrong place

My commands run inside the Claude app, and Windows keeps files they write in your AppData folder in Claude's own private storage. So my silent install landed there, not in your real `AppData\Local\Mimi`. Your desktop shortcut pointed at a program Windows couldn't find, which is why it showed a blank icon. That copy has been uninstalled.

Your history was never in that private storage. It stays in your real folder, and 32 dictations were readable after the uninstall.

The installer also carried three of my test tools, 117 MB in all. They're now kept out, which brought it from 1.55 GB to 1.44 GB.

**Then:** you ran the installer yourself by double-clicking it, and reported "Funcionó bien". That covered the desktop icon with Mimi's face, opening Mimi from it, your history and dictating into other apps. This is your report; I can't see your desktop to check it myself.

## A clean folder, like another PC

Mimi ran hidden from a new folder laid out like the installer's, with only Windows in its search path.

| Check | Result |
|---|---|
| Opens and dictates in English and Spanish | Yes |
| `vulkan-1.dll`, `directml.dll` and the Microsoft C++ files | Loaded from Mimi's own folder |
| Fonts and Mimi's picture | Loaded |
| No graphics driver, Whisper Small in Japanese | Correct text, in 12.4 s, using the processor |

This fixes the problem noted in M3, where a PC without `vulkan-1.dll` couldn't open Mimi. A PC that truly lacks those files hasn't been tested, because this one has them in Windows too. The clean folder proves Mimi's own copies are the ones used.

## Memory while unused

After 15 minutes without dictating, the model leaves memory. It comes back when recording starts. In this test the wait was shortened to 8 seconds.

| Moment | Memory |
|---|---|
| Parakeet ready | 746 MB |
| After release | 71 MB |
| Reloading once recording starts | About 2 s, back to 745 MB |
| Dictation right after the reload | Text 0.18 s after stopping, with about 4 s of speech |

Whisper Turbo also released and reloaded correctly, with text 0.53 s after stopping. A very short dictation right after a release can wait for the model to finish loading.

The 71 MB covers Mimi's main process, not the part that draws its screens.

## Weak PCs, simulated

This PC is an Intel Core i5-11400H with 40 GB of memory. Each run was limited to some of its processor threads, with no graphics card. Its threads are still faster than an old PC's, so a real old PC would be slower.

| Run | Clips finished | Typical wait | Slowest | Per 10 s of speech |
|---|---|---|---|---|
| Parakeet, 12 threads | 50 | 0.85 s | 2.25 s | 0.68 s |
| Parakeet, 4 threads | 50 | 0.99 s | 2.64 s | 0.83 s |
| Parakeet, 2 threads | 50 | 1.63 s | 4.02 s | 1.30 s |
| Whisper Small, 4 threads | 10 in 150 s | 12.62 s | 13.62 s | 7.61 s |
| Whisper Small, 2 threads | 0 in 150 s | | | |

Parakeet stays usable with 2 threads. Whisper isn't practical without a graphics card, so Japanese on a weak PC would be very slow. A PC would also need about 800 MB of free memory.

## Network

| Check | Result |
|---|---|
| Internet connections from Mimi and its 9 web view processes, across 3 hidden runs | 0 |
| Internet libraries in the running app | None. `reqwest`, `ureq` and `hyper` only appear while building it |
| Screen security rules | Block loading anything from outside the app |

### Launches after the silent install

All of Mimi's processes were watched from the moment it opened, with its web view starting fresh. These ran from the copy in Claude's private storage. It's the same program as the one you'll install.

| Launch | Watched | Internet connections |
|---|---|---|
| First install | 120 s | 1, to Microsoft at `150.171.28.11`, port 443 |
| After the fix, reinstalled | 150 s | 0 |

The connection came from the web view's own network part, which is Microsoft's Edge engine that draws Mimi's screens, not from Mimi's code. It was encrypted, so what it sent can't be read. Most likely it was the engine checking for its own updates or settings, but that wasn't confirmed. The fix starts the web view with its background connections, component updates, sync and pings turned off.

**Limits of this check:**

- It looks at open connections about 3 to 5 times a second, so a very brief connection could be missed.
- Each launch was watched once, for up to 2.5 minutes. A connection made later, or only sometimes, wouldn't show.
- Windows itself may still update the web view engine, as it does for every app that uses it.

## Tests

84 unit tests passed. The clean-folder results are in `app-clean.json`, `app-reload-whisper.json` and `app-no-driver.json`, memory is in `memory.csv`, and the weak-PC logs are the `slow-*` files.

## Goals from the plan

| Goal | Result |
|---|---|
| Stop speaking to text after 10 s of speech, under 1 s | Met: 0.68 s |
| Shortcut to listening, under 0.1 s | Not met: the shortcut reacts in about 0.06 s, but your microphone takes 0.1 to 0.7 s to open, and the indicator shows GET READY meanwhile |
| Idle processor use, about 0% | Not measured again on the final app |
| Download without models, under 20 MB | Not met: about 105 MB for the app and its libraries before compression |

## Not tested yet

- Uninstalling
- A real old PC, and a PC without Vulkan files

## Sign-off

You signed off M8 on 2026-11-05 after installing Mimi yourself.

## Not included

- **Code signing:** skipped for personal use. An installer you build on your own PC doesn't trigger the SmartScreen warning, but one copied to another PC could.
