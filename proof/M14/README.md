# M14 proof: plum dark mode, the kanji and the sun and moon switch

Built on 2026-11-05. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

## Your choices

From three proposals (a lighter grey, a dark blue and plum) and two ways to write her name (みみ and 耳), you chose plum and the kanji.

- **Dark mode is now plum:** #2b2231 background and #362c3d cards, never black. The coral and Mimi are unchanged (`dark-home.png`).
- **耳 beside Mimi in both modes.** "Mimi" in kanji, which also means "ear", sits in the empty space to her left, with a small MIMI next to it (`light-home.png`).
- **A sun and moon switch** at the bottom of the left column, above LANGUAGE:
  - The knob slides to the moon for dark mode and to the sun for light mode, with a small bounce.
  - The active side's icon lights up.
  - The track turns coral in dark mode.
  - If Windows is set to reduce animation, it moves without the bounce.
- **The Appearance dropdown in Settings is gone,** because the switch replaces it. "Same as Windows" went with it.

## Checks

| Check | Result |
|---|---|
| Clicking the switch in Mimi's real screens with sample data | Light to dark and back. The colors changed, the knob moved 22 pixels, and the choice was remembered |
| The kanji at the smallest window and maximized | Fits beside Mimi without touching her (`light-smallest.png`, `dark-smallest.png`, `dark-maximized.png`) |
| Unit tests | 107 passed |
| Hidden run of the real app | Dictation and delete all work as before |

Screenshots are drawn with no window, from the real screens with sample data.

## Sign-off

The owner reported "Me gusta" on 2026-11-05.
