# M-Mac-6 proof: start at sign-in and the Mac installer

Built on 2026-11-09. The owner installed Mimi from the `.dmg`, and pasted into Google Chrome from the installed copy.

**Still to confirm:** that Mimi starts by herself after restarting the Mac.

## What changed

- `src-tauri/src/mac/autostart.rs` adds a per-user launch agent that starts Mimi hidden when you sign in. Only a copy in Applications adds itself, so a test build can't take the place of the real Mimi.
- `tools/mac/build_installer.sh` builds the app and makes `Mimi_0.1.0_aarch64.dmg`, with Mimi and a shortcut to Applications. It uses no Finder windows.

## Unit tests

`unit-tests.txt`: 119 passed, 0 failed, 5 skipped, the same five skipped since `proof/M-Mac-1`. The startup tests use a temporary folder, so they never touch the real startup list.

## Checks

| Check | Result |
|---|---|
| The installer | 1.2 GB, passes `hdiutil verify`, signed with "Mimi Local Signing" |
| The app inside it | Valid signature, all speech models included |
| Installed copy | Running from `/Applications`, same build as in the installer |
| Startup entry | Written at 22:40 by the installed copy, valid plist, starts `/Applications/Mimi.app/Contents/MacOS/mimi --hidden` at sign-in |
| Paste from the installed copy | Pasted into Google Chrome at 22:40, with no permission reset |

## Not checked yet

- Mimi starting by herself after a real restart.
- Installing on a different Mac. The signature is the owner's personal certificate, so macOS will warn about the app there.
- The Windows build. These changes were only compiled on the Mac.
