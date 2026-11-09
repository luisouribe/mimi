# M16 proof: the neon poster, in both modes

Built on 2026-11-05. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

## Your choice

You asked for a darker, punk dark mode. You compared:

- the current design
- a neon version
- a shadow version

You chose neon, without the blue glow around Mimi, in both modes. Mimi's illustration is unchanged.

| Part | Light mode | Dark mode |
|---|---|---|
| "mimi" behind her | Hollow letters with a coral-red neon outline and a soft glow | Hollow letters with a pink neon outline and a strong glow |
| The band | Coral with fine screen lines and neon edges | Dark plum to violet with fine screen lines and pink neon edges |
| 耳 | Coral-red with a glow | Pink neon with a glow |
| The shadow under Mimi | A faint coral-red glow | A blurred pink glow |

Screenshots:

- `light-home.png` and `dark-home.png`
- `dark-listening.png`
- `light-smallest.png` and `dark-smallest.png`
- `light-maximized.png` and `dark-maximized.png`
- `dark-settings.png`

## Checks

| Check | Result |
|---|---|
| Smallest and maximized window, both modes | The letters, band and 耳 scale together, and nothing overlaps the side columns |
| Unit tests | 107 passed |
| Hidden run of the real app | Dictation and delete all work as before |

Screenshots are drawn with no window, from the real screens with sample data.

## Sign-off

The owner reported "me encantó" on 2026-11-05.
