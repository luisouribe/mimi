# M5 proof: the Stage design in the real app

Checked on 2026-11-05 on this PC, with no visible window, microphone, clipboard or key presses, then in the real window by you.

**Your check, 2026-11-05:** you went through the real window and reported "Funcionó bien". Mimi's records confirm a dictation pasted into Chrome with its line break (`Hola Juan,` / `Saludos.`), and Mimi had fully closed afterward, as **Quit Mimi** should do.

## Screenshots

These come from the built screens with simulated data, drawn by Edge with no window.

| File | Shows |
|---|---|
| `home.png` | Home at the normal window size |
| `listening.png` | Listening, with the voice bars and Mimi's glow |
| `settings.png` | Settings with Parakeet |
| `settings-whisper.png` | Settings with Whisper Turbo |
| `empty.png` | Home with no dictations yet |
| `still.png` | Mimi with animation off |
| `smallest.png`, `smallest-settings.png` | The smallest window, 1100 x 600 |
| `large.png` | A large window, 1920 x 1040 |

## Checks

| Check | Result |
|---|---|
| Unit tests | 67 passed |
| Hidden real app, Parakeet then Whisper Turbo | Both dictations correct |
| Design fonts load inside the real app, offline | Yes, Oxanium and Manrope |
| Mimi is drawn in a window that isn't active | Yes, 523 pixels wide |
| Animation switch saves off and on | Yes |
| Permission to hide the window | Yes |
| App size | 84.5 MB, up 0.1 MB |

## What changed from the test window

- Mimi's stage, the MIMI word and the side columns now follow proposal A, and all of it scales with the window.
- Mimi's title bar replaces the Windows one.
- The voice bars follow your real microphone, not a loop as in the proposal.
- **Settings** has **Animate Mimi** and **Quit Mimi**.
- The recording indicator uses the design's font.

## Differences from the proposal

- **Snippets button:** left out until M6, so no button leads nowhere.
- **ミミ:** uses Windows' Japanese font at 14 px instead of Noto Sans JP at 12 px, which saves bundling a large font file.
- **Finish hint:** says "Release or tap to finish", because holding the shortcut also works.
- **Status while the model loads:** "Loading".

## A bug caught along the way

The first version hid Mimi's canvas while it loaded, so she was drawn at zero size and could be missing in an inactive window. Two screenshots showed it. It's fixed, and the hidden test now checks her size.

## Not tested yet

- Processor and graphics use on this design. The animation code is unchanged from M1's measured version, which used about 1.7% processor and 5% graphics while active

There's no video in this proof, because recording the real window would need your desktop. Opening the real app shows the motion better.
