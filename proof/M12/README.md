# M12 proof: shortcuts inside Mimi's own window

Built on 2026-11-05. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

## What the diagnostic log showed

After your test, `diagnostics.log` in Mimi's data folder showed:

- With Chrome in front, Ctrl + Space reached Mimi: it started, and stopped on release.
- With Mimi's window in front, no shortcut key was noted at all. Only your two mouse clicks on the button were.

So while her own window was in front, Mimi either didn't receive the keys, or received them but decided Ctrl wasn't held. She used to ask Windows whether Ctrl was down at the moment Space arrived. That's the likely cause, because the answer Windows gives depends on which window is in front.

## The fix, in two layers

1. **Mimi follows Ctrl, Alt and Shift herself,** from the presses and releases she sees, instead of asking Windows. A modifier whose release went missing, for example across Win + L, is forgotten after 1.6 seconds, because Windows keeps repeating a key that's really held.
2. **A backup inside Mimi's window.** If the shortcut keys reach her screen anyway, the screen passes them on, and they work the same way. This also covers changing a shortcut while her window is in front. When the first layer catches the keys, it hides them, so they're never handled twice.

The log stays on and now also notes which layer caught each shortcut. If something still fails, it shows why. It holds only key names, times and numbers, never words.

## Checks

| Check | Result |
|---|---|
| Unit tests | 107 passed, up from 104. New: a forgotten modifier, a repeating one kept, and shortcuts from Mimi's window |
| Hidden run of the real app | Dictation and delete all work as before |
| The backup, in Mimi's real screens with simulated keys inside the page | Hold Ctrl + Space with Ctrl let go first: start then stop, and the Space never reached the focused button (0 clicks) |
| | Ctrl + Shift + Space twice: start, then stop |
| | A plain Space and AltGr + Space: nothing |
| | Changing a shortcut: Alt + D captured |

**Not checked by me:** real key presses with Mimi's window in front, which need your keyboard.
