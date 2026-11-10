# M-Mac-2 proof: global shortcuts on Mac

Built on 2026-11-09. **Signed off by the owner on 2026-11-09 ("Funcionaron")**, after using the shortcuts with their own keyboard and voice.

**Replaced in M-Mac-3:** the event tap described here became system hot keys, which need no permission. See `proof/M-Mac-3/README.md`.

## What changed

- Hold Option + Space to talk, or tap Option + Shift + Space to start and stop. The owner chose these, because macOS keeps Ctrl + Space for switching keyboard languages.
- `src-tauri/src/mac/hotkey_tap.rs` listens through a Quartz event tap and feeds the same `HotkeyLogic` as Windows.
- A saved `settings.json` that still has Windows' two default shortcuts switches to the Mac ones when it loads.
- Mimi needs the Accessibility permission for this. She asks once, and starts listening as soon as it's granted, with no restart.
- The screens and the tray say "Option" in place of "Alt" on Mac.

## Unit tests

`unit-tests.txt`: 113 passed, 0 failed, 5 skipped, the same five skipped in `proof/M-Mac-1`.

The new tests feed Mac key events through the shortcut logic. They cover holding, letting go of Option first, tapping, a repeating Space, ordinary typing passing through, Ctrl + Space staying free, and a missed Option release.

## From the owner's test

These numbers are from Mimi's logs, which never contain words.

| Check | Result |
|---|---|
| Shortcut to recording, last four uses | 62 to 66 ms. The goal is under 100 ms |
| Microphone ready | 117 to 121 ms |
| Speech heard in the three real dictations | 3.0, 4.1 and 7.0 seconds |
| Speech recognition | 148 to 259 ms |

## Not built yet

Text still goes only to Mimi's history. Pasting into other apps, the floating indicator, the chimes, quieting other apps, start at sign-in and the installer are still to come.

Each rebuild has a new ad-hoc signature, so macOS may ask for the microphone and Accessibility again, or a permission that looks on may need turning off and on.
