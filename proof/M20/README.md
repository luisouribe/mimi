# M20 proof: lists written as digits

Built on 2026-11-09. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

The example sentences here are invented. They're written the same way as the owner's real test dictations, which stay private.

## Why

Your two list tests on 2026-11-09 stayed as plain text, on the M19 version. The list rules hadn't got worse. Parakeet wrote the same kind of speech differently that day:

| What Parakeet wrote | Why it was missed |
|---|---|
| "…que hacer hoy 1 lavar las toallas … centro 2 hornear el pastel … capas Y 3 tratar de arreglar…" | Digits, with no punctuation. Mimi only accepted a digit with a pause after it, at the start of a clause |
| "…tendría que ser 1 buscar mejores plantas … Dos, el patio trasero … Y tres que el encargado…" | The same "1", followed by words. A digit and a word couldn't be part of one count |

## What changed

- Digits count anywhere in a sentence, with or without a pause.
- Digits and words can mix: "1 … Dos, … Y tres …".
- "Primero … segundo … tercero" counts with no punctuation at all.
- Numbers that are amounts stay as they are, including when they appear in the same run as a list marker.

These stay ordinary sentences, and each one is now a test:

- "Compré 1 kilo, 2 litros y 3 latas."
- "Compré 1 pan, dos leches y tres huevos."
- "Tengo 1 hijo y 2 hijas."
- "Vi 1 perro 2 veces en el parque."
- "El 1 de mayo, el 2 de junio y el 3 de julio salimos."
- "Capítulo 1 habla de perros, capítulo 2 habla de gatos y capítulo 3 habla de aves."
- "A las 2 comemos y a las 3 dormimos."
- "Primero, comprar 2 panes y luego dormir."

## A new kind of test

Each earlier fix answered one way Parakeet had written a list. The new test writes one spoken list in 44 different ways: ordinals, plain numbers, digits and a mix, with and without pauses, with no punctuation, commas or full stops between items, and with or without "y" before the last one. All 44 have to give exactly the same list.

Four more combinations are left out on purpose: plain words ("uno … dos … tres") with no punctuation at all. Those read too much like ordinary talk.

## How those two come out now

```
A ver entonces vamos a hacer la lista que cosas que tengo que hacer hoy:
1. Lavar las toallas del one stop del centro.
2. Hornear el pastel grande 6 capas.
3. Tratar de arreglar la bici para Luna.
Y ya Eso es todo por hoy
```

```
Entonces, para los cambios que hay que hacer en la casa nueva tendría que ser:
1. Buscar mejores plantas para el jardín y ver si ellas son posibles causas del poco verde.
2. El patio trasero revisar que los tubos estén bien sellados.
3. Que el encargado de la mudanza sea una persona que está capacitada para llevar los muebles a cabo.
Y ahí se acabó la lista.
```

Misheard words come from the speech model. Mimi doesn't rewrite them.

## Checks

| Check | Result |
|---|---|
| List tests, including two sentences written like your dictations, the 44 ways of writing a list and the 8 sentences above | All pass |
| All unit tests | 114 passed |
| 537 different texts: earlier tests plus every dictation in your history | 19 became lists, all meant to: the list tests and your four list attempts. No ordinary dictation changed |
| Hidden run of the real app, no window | The spoken list came out right. Dictation, snippets, history search, delete and clear work as before (`app-final.json`). No volume memory was written |

**Signed off:** on 2026-11-09 you installed this version, tested it with your own voice and confirmed it works. Parakeet may still write a list in a way not covered here.

The history check read your dictations on this PC only, read-only. Nothing from it was copied into the project.
