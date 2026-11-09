# M11 proof: shortcuts, maximize and languages

Built on 2026-11-05. The new installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

## The shortcut didn't work inside Mimi's window

Mimi's own log of your dictations showed it. One recording started with the shortcut inside Mimi's window and stopped 0.08 seconds later.

The cause: when you let go of Ctrl before Space, Mimi stopped hiding the Space key. Inside her window, that Space release reached the "Start talking" button, which had focus, and pressed it. By then the button said "Stop and type", so it stopped the recording at once. In other apps a lone Space release does nothing, which is why it only failed in Mimi.

Now the release of a shortcut's main key is always hidden, whatever order you let go of the keys. This is a unit test: `releasing_ctrl_first_stops_and_the_space_release_stays_hidden`.

## Two shortcuts you can change

Settings has a new Shortcuts section (`shortcuts.png`) with two shortcuts:

| Shortcut | Default | What it does |
|---|---|---|
| Hold to talk | Ctrl + Space | Records while you hold the keys. Letting go finishes |
| Tap to start and stop | Ctrl + Shift + Space | One press starts, the next press finishes |

- **Changing one:** press CHANGE, then the new keys (`shortcut-waiting.png`). Esc or CANCEL keeps the old one.
- **Getting the defaults back:** "Use the original shortcuts".
- **Stopping a recording:** either shortcut stops it, whichever started it. So does Mimi's button.
- **What Home shows:** both shortcuts with what each one does (`home.png`, `listening.png`).

Mimi refuses keys that would get in the way of typing, and says why:

- a letter, number or Space on its own, or with only Shift
- Ctrl + Alt with a letter or symbol, because on Spanish keyboards that's AltGr, which types @ and #
- Alt + Tab, Alt + F4, Ctrl + Esc and the copy and paste shortcuts
- anything with the Windows key
- the same keys as the other shortcut

Keys like F8, Alt + D or Ctrl + Alt + Space are fine. With Alt shortcuts, Mimi also stops Alt from opening the menu bar in apps like Notepad.

**Change from before:** Ctrl + Space used to do both jobs, depending on how long you pressed it. Now it only holds, as you asked. A quick tap of Ctrl + Space records almost nothing and shows "No speech heard".

## Maximize

The title bar has a maximize button between minimize and close. While maximized it changes to a restore button. On large or maximized windows, the right column grows a little (`home-maximized.png`, `settings-maximized.png`).

## Which languages each model understands

Settings lists every language under the speech model (`languages.png`). Each one opens with a click.

| Model | Languages |
|---|---|
| Parakeet V3 | 25 European languages, Spanish and English included. No Japanese, Chinese or Korean |
| Whisper Turbo | 100 languages, Japanese, Chinese and Korean included |
| Whisper Small | 99, the same as Turbo without Cantonese |

The lists come from the models' own documentation. A note says Whisper is much less accurate in rare languages.

## Checks

- Unit tests: 104 passed, up from 95. 20 of them cover the shortcuts, including the bug and the refused keys.
- Hidden run of the real app, with no window: dictation, cleanup, snippets and delete all work as before.
- Screenshots were drawn with no window, from the real screens with sample data.

**Not checked by me:** pressing real keys. My tests never press keys on your desktop, so you're the one who checks that the shortcuts work, inside Mimi and in other apps.
