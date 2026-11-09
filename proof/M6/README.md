# M6 proof: snippets and app icon

Checked on 2026-11-05 on this PC, with no visible window, microphone, clipboard or key presses, then with your voice.

**Your check, 2026-11-05:** you repeated the snippet test on this version and reported "Funcionó bien".

Say a saved phrase, and Mimi replaces it with the saved text, wherever it appears in the dictation. These are plain word comparisons, with no AI.

## Why the rule changed

The first version only inserted a phrase said on its own, with punctuation around it. In your test, all four dictations failed, because you said it inside a sentence and Parakeet didn't always mark your pauses. Windows' voices had passed because they paused sharply and were given the phrase in isolation. You chose to insert it anywhere.

## Your four dictations, with the new rule

These tests use `Calle Luna 5`, and run through the rules only, not your voice.

| What Parakeet wrote | Now becomes |
|---|---|
| `Hola Juan. Manda la mi dirección. Saludos.` | `Hola Juan. Manda la Calle Luna 5. Saludos.` |
| `Hola Juancy, puedes mandarla a mi dirección, saludos` | `Hola Juancy, puedes mandarla a Calle Luna 5, saludos` |
| `Hola, sí Puedes mandarla a mi dirección Gracias Juan saludos` | `Hola, sí Puedes mandarla a Calle Luna 5 Gracias Juan saludos` |
| `Hola Juan, si puedes mandarlo porfa mi dirección Gracias por el mensaje, saludos` | `Hola Juan, si puedes mandarlo porfa Calle Luna 5 Gracias por el mensaje, saludos` |

## Real app, hidden

Spoken by Windows' offline voices through Parakeet V3 and Whisper Turbo.

| Result | Parakeet V3 | Whisper Turbo |
|---|---|---|
| Phrase said, alone or inside a sentence: text inserted | 10 of 10 | 10 of 10 |
| Only similar words: left alone | 6 of 6 | 6 of 6 |

The similar sentences include "Mi nueva dirección es Calle Luna 5", "La dirección de mi casa es muy grande" and "Please check my emails today". Results are in `app-final.json`.

## Accidental inserts

| Check | Result |
|---|---|
| 700 texts from the M3 reading recordings, against 16 likely phrases | 0 inserts |

That recording set is mostly readings and poems, so it's a limited guide to everyday speech.

**The trade-off you chose:** the exact phrase always fires, even in normal speech. "Did you read my email yesterday?" becomes "Did you read luis@example.com yesterday?" A phrase you don't otherwise say avoids this, so "dirección de casa" is safer than "mi dirección".

Mimi refuses:

- A single word, except in languages written without spaces
- A line break command such as "nueva línea"
- A phrase another snippet already uses

## Other checks

| Check | Result |
|---|---|
| Unit tests | 82 passed |
| Adding, changing and deleting | Tested |
| Your existing history, upgraded | All 24 dictations kept |
| Capitals, accents and hyphens | Ignored, so "MI DIRECCION" and "My e-mail" both match |

## App icon

Mimi's face on the coral background, with the design's angled corners. Checked by eye at 256, 64, 48, 32, 24 and 16 pixels, and read back from the built `mimi.exe`. Source: `design/assets/mimi-icon.png`, made by `tools/art/make_icon.ps1`.

## Known limits

- **Misheard phrases:** the phrase must be recognized correctly, so unusual words or names may fail.
- **A period after a link:** a link in the middle of a text keeps the period the model writes, as in `https://example.com/mimi.`
- **Privacy:** snippets and history are stored without encryption on this PC, so don't save passwords in them.
- **Dictating inside Mimi's own window:** the text goes to history only, so it won't fill the snippet form.

## Not tested yet

- Japanese phrases by voice
