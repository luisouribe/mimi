# M18 proof: lists as people really count

Built on 2026-11-06. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

The example sentences here are invented. They're written the same way as the owner's real test dictations, which stay private.

## Why

Your first test of M17 made no lists. Mimi was the new version and cleanup was on, but the M17 rules expected tidier counting than you used.

| Dictation | Why M17 missed it |
|---|---|
| "hola ok bueno necesito hacer una lista primero regar las plantas y dos pasear al perro de nuevo y tres ver si ya llegó el paquete" | Parakeet wrote no punctuation at all. "Primero" was followed by "dos" and "tres", and M17 needed the same kind of word throughout |
| "A ver, lo que necesito son dos cosas, uno comprar las flores y dos, pintar la cerca." | There was no pause after "uno". This one was dictated at 12:18, before the M17 version was installed |
| A recipe: "Arroz, frijol, sal, limón." | There was no counting and no "viñeta", so it isn't a spoken list. It stays an ordinary sentence on purpose. Otherwise every list of things in a normal sentence would break into lines |

## What changed

- **Mixed counting.** "Primero … dos … tres" counts, and so does "first … two … three". Spanish and English are still kept apart.
- **No pauses needed** when you count to three or more: "primero ver si jala y dos ver si corre y tres …".
- **Two items** count without pauses when the second number has a pause after it: "uno comprar las flores y dos, pintar la cerca".
- **The first number may be mid-sentence**, as in "necesito hacer una lista primero ver …". The numbers after it must follow "y" or a pause.
- **A number inside an item doesn't break the list.** "Primero, compra uno. Segundo, llama a Ana." stays a two-item list.
- **New guard.** Counting with nothing in between, as in "uno, dos, tres, probando", is no longer a list. M17 would have turned it into one.

Those two now come out like this:

```
hola ok bueno necesito hacer una lista:
1. Regar las plantas.
2. Pasear al perro de nuevo.
3. Ver si ya llegó el paquete.
```

```
A ver, lo que necesito son dos cosas:
1. Comprar las flores.
2. Pintar la cerca.
```

These still stay ordinary sentences: "Primero voy a comer y dos horas después me duermo", "Primero, quiero agradecer. Y dos personas vinieron", "Primero comimos y segundo nos fuimos" and "Es uno de los mejores y dos de ellos ganaron".

## Checks

| Check | Result |
|---|---|
| List tests, including two sentences written like your dictations and 5 new sentences that must stay unchanged | All pass |
| 530 different texts: 482 from earlier tests and every dictation in your history | 14 turned into lists, and all 14 were meant to: the list tests and your two attempts. None of your other dictations changed |
| All unit tests | 111 passed |
| Hidden run of the real app, no window | The spoken list clip came out as the numbered list. Dictation, snippets, history search, delete and clear all work as before (`app-final.json`). No volume memory was written, so the owner's apps were never lowered |

The history was read on this PC only, read-only, and only to run this check. Nothing was copied into the project.
