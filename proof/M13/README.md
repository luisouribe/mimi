# M13 proof: dark mode

Built on 2026-11-05. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

## What's new

Settings has a new Appearance choice at the top of the switches (`dark-appearance.png`, `light-appearance.png`):

| Choice | What it does |
|---|---|
| Light | The design as before, and still the default |
| Dark grey | Dark grey backgrounds, never black, with the same coral and Mimi |
| Same as Windows | Follows Windows' light or dark app mode, and changes when Windows does |

The dark colors:

| Part | Color |
|---|---|
| Background | #2a2c31, a dark grey |
| Cards and fields | #33363d, a slightly lighter grey |
| Dots, lines | #3d4047, #5c616b |
| Text | #eceef2 |
| Notes | #a7abb4 |
| Coral, MIMI word | Unchanged |

The choice is remembered and applied before the window appears, so dark mode never flashes white. Dropdown lists and scroll bars turn dark too. The recording indicator was already dark and is unchanged.

## Screenshots

- `dark-home.png`
- `dark-listening.png`
- `dark-settings.png` and `dark-settings-lower.png`
- `dark-delete-confirm.png`
- `dark-snippets.png`
- `dark-smallest.png`, at the smallest window size

They're drawn with no window, from the real screens with sample data.

## Checks

- Unit tests: 107 passed, including the saved Appearance choice.
- Hidden run of the real app: dictation and delete all work as before.
- Text and notes are easy to read on the dark grey. Light text on dark grey has a contrast well above what's needed for reading. The coral button and the red "delete all" button use dark text, so both stay readable.
