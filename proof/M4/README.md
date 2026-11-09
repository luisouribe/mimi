# M4 proof: cleanup by rules

Measured on 2026-11-04 on this PC, with no visible window, microphone, clipboard or key presses, then checked with your voice.

The rules are plain lists and word comparisons, with no AI. They run after recognition, and the code is in `mimi/src-tauri/src/tidy.rs`.

## Before and after

These came through the real speech models from Windows' offline voices, not a person.

| What the model wrote | What Mimi gives you |
|---|---|
| `Um, I think we should, uh, leave early.` | `I think we should leave early.` |
| `Eh, creo que, em, deberíamos salir temprano.` | `Creo que deberíamos salir temprano.` |
| `I I I I I I I I I I I I think that the plan is very very good.` | `I think that the plan is very very good.` |
| `Yo, yo, yo creo que el plan es muy, muy bueno.` | `Yo creo que el plan es muy, muy bueno.` |
| `I think we should go. New line. See you tomorrow.` | `I think we should go.` / `See you tomorrow.` |
| `Hola Juan, nueva línea, gracias por tu mensaje.` | `Hola Juan,` / `Gracias por tu mensaje.` |
| `The report is ready, new paragraph, please review it today.` | `The report is ready.` / blank line / `Please review it today.` |
| `Llegamos tarde. Punto y aparte. Mañana seguimos.` | `Llegamos tarde.` / blank line / `Mañana seguimos.` |

## Left alone on purpose

| Text | Why |
|---|---|
| `We launched a new line of products this year.` | No pause around the words |
| `Lanzamos una nueva línea de productos este año.` | No pause around the words |
| `Lanzamos logo, nueva línea, nuevo empaque y web.` | Looks like a list |
| `Es muy muy bueno.` / `No, no, no.` / `Gracias, gracias.` | Repeated on purpose |
| `I think that that works.` / `He had had enough.` | Correct grammar |
| `Por favor por favor.` / `Paso a paso a paso.` | Set phrases |
| `Está bien, ¿eh?` / `Este libro es bueno.` / `Use an em dash.` | Real words |
| Japanese text | No rules for Japanese yet |

## How much of a pause a command needs

Times a spoken command became a line break, by pause length. Whisper Turbo used the clip's language unless noted.

| Model | 100 ms | 200 ms | 300 ms | 500 ms | 800 ms |
|---|---|---|---|---|---|
| Parakeet V3 | 5 of 5 | 5 of 5 | 5 of 5 | 5 of 5 | 5 of 5 |
| Whisper Turbo | 5 of 5 | 5 of 5 | 5 of 5 | 5 of 5 | 5 of 5 |
| Whisper Turbo, detect language | 5 of 5 | 5 of 5 | 5 of 5 | 5 of 5 | 5 of 5 |
| Whisper Small | 4 of 5 | 5 of 5 | 5 of 5 | 5 of 5 | 5 of 5 |

- **Result:** 79 of 80 commands worked. The miss was Whisper Small mishearing "nuevo párrafo" as "nuevo para fo".
- **No pause at all:** the command doesn't work, and the words stay in the text.
- **False commands:** none in 16 results from sentences that only contain the words.

## Checks

| Check | Result |
|---|---|
| Unit tests, whole app | 67 passed |
| 1,400 clean texts from the M3 reading recordings, including their poems | 0 changed by mistake. 6 checks of 3 clips gained a missing space before `¿` |
| Real app, hidden, on Parakeet and Whisper Turbo | Saved cleaned text and line breaks in the history. See `app-final.json` |
| Time the rules add per dictation | 0.15 ms at most in 8 dictations |

The 1,400 check found a real bug in an earlier version, which deleted deliberate repeats such as "perdón perdón". Words said twice now collapse only if they're on a list of words people trip over.

## What your first try showed

Your first try ran on the previous copy of Mimi, so the rules never ran. Your dictations still exposed two bugs in the rules, both now fixed:

- Parakeet can write `Línea,¿cómo` with no space, which hid the command.
- A command right after a question mark was refused.

Your second try, on the corrected copy, passed steps 1, 4 and 5 and exposed two more problems, both now fixed:

- Parakeet wrote "muy" 11 times when you said it twice, the same way in both tries. A word that's fine when repeated now goes back to two once it appears 4 times or more. Numbers and laughter keep any count.
- Parakeet wrote `nueva línea Gracias`, with no punctuation after the command. A capital right after a lowercase command now counts as a new sentence.

**Your final check, 2026-11-05:** you repeated both steps on the latest copy and reported "Funcionó bien". History shows the line break correct, and "el el" and the stuck "muy" gone. Both dictations stayed in Mimi's own window. During M5 a dictation pasted into Chrome with its line break, and you reported it working.

You didn't decide whether "este" should be removed, so it stays.

## Known limits

- **Filler words that are also real words stay:** "este", "o sea", "pues", "bueno", "like" and "you know" are never removed, because the rules can't tell which use is which.
- **Some stutters stay:** "is is" stays because it can be correct, and "he he" stays because it can be a laugh. A word said twice that isn't on the list also stays.
- **Some real uses of "um" are removed:** "Mr. Um" and "um dot com" lose the word.
- **Portuguese and German on Parakeet:** "um" is a real word there and would be removed. Turn off **Clean up my speech** in Settings, or use Whisper with that language chosen.
- **Four deliberate repeats become two:** "no no no no" comes out as "no no".
- **A brand right after a command-like phrase:** "Compramos, nueva línea Nike" would break the line.
- **Mid-list commands:** a list can still trigger a command if it doesn't look like one, and "thanks and bye" right after a command between commas blocks it.

## Not tested yet

- The new Settings switch and the history's line breaks, on screen
