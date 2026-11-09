# M9 proof: "este", start with Windows, delete all dictations

Built and checked on 2026-11-05. The new installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe` (1.44 GB).

## "Este" as a hesitation

"Este" is removed only when it's a hesitation, never when it means "this". Clean up my speech must be on, as it is by default.

It's removed when:

- a pause cuts it off: "Este, quería…", "Fui a la, este, tienda", "Pues este… no sé"
- it stands alone as a sentence before more words: "Este. Quería preguntarte algo."
- it follows a word that "this" can't follow: "Fui a la… este tienda"
- it's drawn out: "Esteee"
- no pause is written, but the next word can't follow "this": a person ("yo", "él"), an article ("el", "un"), a word that starts a new thought ("ahora", "pues", "bueno") or an "I" or "we" verb ("quería", "quiero", "vamos"). "Quiero este ahora" keeps it, because "quiero" takes "este" as what's wanted.

### Your first test, and the fix

Your dictation "Este quería preguntarte algo." kept "este", because Parakeet wrote no comma after it and the rule needed one. The same happened in "…de mimi este ahora vamos a ver…". Both are now cleaned, and are unit tests.

It stays in "Este libro", "Me gusta este.", "Quiero este, no ese.", "Ese, este y aquel", "¿Este?", "Vivo en el este" and "¿Cuál quieres? Este."

### Synthetic Spanish voice through both models

| Said | Parakeet wrote | After cleanup | Whisper Turbo wrote | After cleanup |
|---|---|---|---|---|
| Este / quería preguntarte algo | Este. Quería preguntarte algo. | Quería preguntarte algo. | ¿Este? Quería preguntarte algo. | Unchanged |
| Fui a la / este / tienda de la esquina | Fui a la... este tienda de la esquina. | Fui a la tienda de la esquina. | Fui a la... este... tienda de la esquina. | Fui a la tienda de la esquina. |
| Pues este / no sé qué decirte | Pues este, no sé qué decirte. | Pues no sé qué decirte. | Pues este... no sé qué decirte. | Pues no sé qué decirte. |
| Hola Juan / Este / te llamo mañana | Hola Juan. Este. Te llamo mañana. | Hola Juan. Te llamo mañana. | Hola Juan. Este, te llamo mañana. | Hola Juan. Te llamo mañana. |
| Este libro es muy bueno | Unchanged | Unchanged | Unchanged | Unchanged |
| Quiero este / no ese | Quiero este. No ese. | Unchanged | Quiero este, no ese. | Unchanged |
| Este año fue muy bueno para todos | Unchanged | Unchanged | Unchanged | Unchanged |
| Me gusta más este | Unchanged | Unchanged | Unchanged | Unchanged |

| Este quería preguntarte algo (no pause) | Unchanged | Quería preguntarte algo. | Unchanged | Quería preguntarte algo. |
| …la aplicación de Mimi, este ahora vamos a ver… | …de Mimi. Esta ahora vamos a ver… | Unchanged | …de Mimi, este ahora vamos a ver… | …de Mimi, ahora vamos a ver… |
| Quiero este ahora | Unchanged | Unchanged | Unchanged | Unchanged |
| Este no me gusta | Unchanged | Unchanged | Unchanged | Unchanged |

Parakeet: 11 of 12 handled correctly. It heard the synthetic voice say "Esta" in the long sentence, which isn't touched, because "está ahora" is real Spanish when the accent is missing. Earlier it was 8 of 8. Whisper Turbo: 11 of 12. It wrote the first one as a question, "¿Este?", which is kept on purpose because a real "¿Este?" means "This one?".

The voice is Windows' own and says "este" more evenly than a person does, so a real hesitation may come out written differently.

### Real texts

All 112 saved texts from earlier tests that contain "este" were run through the new rule. None lost the word.

## Start with Windows

A new switch in Settings, on by default. When on, Mimi adds herself to Windows' startup list, the same one Task Manager's Startup apps page shows. At sign-in she starts in the tray with her window closed, ready for Ctrl + Space. Opening her from the desktop icon or the tray shows the window.

- Only the installed copy adds itself, so a test build can never replace the real Mimi.
- Uninstalling removes the entry, because it uses the installer's name, "Mimi".
- Tested with unit tests on a test entry in the registry, not on the real startup list.

**Not checked yet:** a real sign-in after installing. My commands run inside the Claude app, whose registry changes Windows keeps apart, so only you can see this work.

## Delete all dictations

A new button in Settings. It asks first, with the number of dictations, and says snippets stay. "There are no dictations to delete" shows when the history is already empty.

After deleting, Mimi rewrites the history file, so the old words aren't left inside it.

| Check | Result |
|---|---|
| Unit test, 300 dictations with a test word | All deleted, the word found nowhere in the file, the snippet kept, new dictations work |
| Hidden run of the real app | 2 dictations deleted, 0 left, 1 snippet kept |

Screenshots: `settings-bottom.png`, `clear-confirm.png`, `clear-confirm-smallest.png`, `clear-nothing.png`.

## Sign-off

You reinstalled with the fixed rule and reported "Funcionó bien" on 2026-11-05. This is your report, since I can't see your desktop. Starting at sign-in is only proven once you've restarted or signed in again with Mimi installed.

## Tests

89 unit tests passed, up from 84. The hidden app run turned the three hesitation clips into "Quería preguntarte algo." twice and "Pues no sé qué decirte.", and kept "Este libro es muy bueno." Deleting all removed its 3 dictations and kept the snippet.

**Known limit:** with no pause written, "este" before a word not on the list stays, for example "Este creí que…". Tell me any phrase where it's left behind, and it can be added. The results are in `app-final.json`, `este-parakeet.json` and `este-turbo.json`.
