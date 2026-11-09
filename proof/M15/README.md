# M15 proof: 耳 in the title bar, lowercase mimi, and a recording indicator that follows the mode

Built on 2026-11-05. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

## Your choices

You compared three versions and chose C:

- **Title bar:** 耳 in coral replaces the katakana ミミ beside the mimi logo.
- **The word behind Mimi:** "mimi" in lowercase, like her logo and her top. The small word next to the big 耳 is lowercase too.

Screenshots: `light-home.png`, `dark-home.png`, `light-smallest.png`, `dark-maximized.png`.

## The recording indicator follows light or dark mode

The bar that appears over other apps while you dictate now matches Mimi's window (`compare-indicator.png`):

| Mode | Indicator |
|---|---|
| Light | Almost-white bar, dark text, coral voice bars |
| Dark | Plum bar like the dark mode, light text, pink voice bars |

- **On any app:** a soft shadow keeps the bar visible on both white and dark apps.
- **When you flip the switch:** the indicator changes at once, with no restart.
- **"Typing" and the notices:** these use the same colors.

## Checks

| Check | Result |
|---|---|
| Indicator pictures from its real page, with simulated sound, on white and dark backgrounds | Readable in all 6 |
| Unit tests | 107 passed |
| Hidden run of the real app | Dictation and delete all work as before |

Screenshots are drawn with no window.
