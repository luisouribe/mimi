# M17 proof: spoken lists

Built on 2026-11-06. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

## What's new

You chose two ways to make a list, out of three offered: counting out loud, and saying "viñeta". Both work in Spanish and English, as part of "Clean up my speech". The note under that switch in Settings now mentions them.

**Counting** makes a numbered list.

- **Ordinals:** primero, segundo, tercero… up to décimo; first, second, third…
- **Fuller forms:** "en primer lugar", "primero que nada", "first of all"
- **Plain numbers:** uno, dos, tres; one, two, three; and digits

"Por último", "finalmente", "finally" or "lastly" can end the list as the next number.

> "Para mañana necesito, primero, comprar pan, segundo, llamar a Juan, y tercero, mandar el video."

```
Para mañana necesito:
1. Comprar pan.
2. Llamar a Juan.
3. Mandar el video.
```

**"Viñeta"**, or "bullet" or "bullet point", before each item makes bullets.

> "Ingredientes, viñeta, harina, viñeta, huevos, viñeta, leche."

```
Ingredientes:
- Harina
- Huevos
- Leche
```

**How the text is shaped:**

- A short sentence just before the list ends with a colon.
- Each item starts on its own line, with a capital letter.
- An item of two or more words ends with a full stop. A single word doesn't.
- Text after the list starts on a new line.
- One "viñeta" at the start of a dictation makes a single bullet. That's useful for adding items one dictation at a time.

**What doesn't become a list, on purpose:**

- **Counting that doesn't go 1, 2 in order.** "Primero voy a comer y luego me duermo" stays as it is.
- **A plain number with no pause after it.** "Compré dos panes y tres leches" stays.
- **A number that doesn't start a clause.** "Llegó en segundo lugar" and "El primero, el segundo y el tercero llegaron" stay.
- **"Viñeta" or "bullet" used as a word.** "Esta viñeta del cómic", "the bullet train" and "a bullet hit the wall" stay.

## Checks

| Check | Result |
|---|---|
| Unit tests | 111 passed, up from 107. New: 4 list tests with 34 sentences |
| 6 spoken lists and 4 ordinary sentences, synthetic voices, through Parakeet and Whisper Turbo | 20 of 20 right after the fixes below. Results in `lists-parakeet.json`, `lists-turbo.json` and `lists-cleaned.json` |
| Every saved text from earlier tests, 5,748 of them | None turned into a list by mistake |
| Hidden run of the real app, no window | The spoken lists came out as a numbered list and a bulleted list, as in the examples above (`app-list.json`). Dictation, cleanup, snippets and delete all work as before (`app-final.json`) |

Two fixes came from the voice test:

- **No punctuation.** Parakeet wrote "Shopping list Bullet Point Apples Bullet Point Bread Bullet Point Milk", with no punctuation at all. Saying the bullet word twice or more now counts without pauses.
- **A full stop before the list.** Both models wrote the pause before a list as a full stop: "Ingredientes." After a short sentence of up to 8 words, it now becomes a colon.

The voices are Windows' own and say lists more evenly than a person does. Your real dictations may come out with different punctuation.

In `app-final.json`, the list clip ends at "Y tercero." because that run recorded only 9 seconds of an 11-second clip. That was a mistake in the test settings. The run in `app-list.json` used the full clip.
