# M-Mac-3 proof: pasting into other apps on Mac

Built on 2026-11-09. **Confirmed by the owner on 2026-11-09 ("si ya sirvió")**, and by Mimi's own record of a dictation pasted into Google Chrome.

## What changed

- `src-tauri/src/mac/paste.rs` pastes with Cmd + V into the app in front, and saves and restores the clipboard around it.
- Dictated text is marked temporary, so clipboard history apps skip it.
- If you switch apps while Mimi works, she doesn't paste into the wrong one. The text is left copied, ready for Cmd + V.
- The history now records the app's name, never a window title.
- The global shortcuts changed from a Quartz event tap to system hot keys, in `src-tauri/src/mac/hotkey_carbon.rs`. Hot keys need no permission, so they work outside Mimi even when Accessibility isn't valid.

## Unit tests

`unit-tests.txt`: 115 passed, 0 failed, 5 skipped, the same five skipped since `proof/M-Mac-1`.

The clipboard tests use a private pasteboard, so they never touch the owner's real clipboard.

## From the owner's test

| Check | Result |
|---|---|
| Option + Space from Google Chrome | Shortcut to recording in 61 ms |
| Stop speaking to text, 3.3 seconds of speech | 163 ms |
| Outcome | Pasted into Google Chrome and saved in the history |
| Paste time | 429 ms, which includes the 400 ms wait before the clipboard is restored |

Only the first paste after the permission was fixed is on record: one dictation, in one app.

## The problem that took longest

The Accessibility list showed Mimi switched on, but pasting was still blocked. macOS's own log said `Failed to match existing code requirement` for `kTCCServiceAccessibility`.

Mimi is signed ad hoc, so macOS ties each permission to one exact build. After a rebuild, the old entry stays visible and switched on but no longer applies, and switching it off and on doesn't replace it.

**The fix:** run `tccutil reset Accessibility com.mimi.dictation`, then add Mimi to the list again. Any new build needs this again until the app has a stable signature.

## Not checked yet

- Pasting in apps other than Google Chrome.
- The Windows build. These changes were only compiled on the Mac.
