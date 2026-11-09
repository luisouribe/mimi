# M19 proof: ending a list, and numbers said twice

Built on 2026-11-06. The installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe`.

The example sentences here are invented. They're written the same way as the owner's real test dictations, which stay private.

## Why

In your M18 test the list worked, but two things went wrong:

```
3. Tres que el encargado de la mudanza sea una persona que esté capacitada
para llevar los muebles ya de esa manera hay que ver la casa nueva.
```

- **"Tres que".** You said the number twice, as in "tercero… tres que el encargado". Every way Parakeet could have written that gave the same result, so the repeat came from the speech, not from the rules.
- **The list didn't end.** You said "y ya" to close it, but Parakeet wrote only "ya" and dropped the "y". On its own, "ya" is far too common to end a list.

## What changed

**Ways to end a list.** The words stay and only a new line is added before them:

| Say | Result |
|---|---|
| "y ya", "y listo", "eso es todo", "eso sería todo", "and that's it", "that's all" | The last item ends and the rest starts on a new line: "Y ya de esa manera…" |
| "nueva línea" (already worked) | The rest starts on a new line |
| "punto y aparte" (already worked) | The rest starts a new paragraph. This one always works, even when the model drops a word, because it's a spoken command |

"Y ya que estás ahí…" means "since you're there", so it doesn't end the list.

**A number said twice is written once.** "Tercero, tres que el encargado…" becomes "3. Que el encargado…". A quantity stays: "Tercero, tres cucharadas de azúcar" becomes "3. Tres cucharadas de azúcar".

**Fixed along the way.** A list whose items start with quantities, such as "Primero, dos tazas de harina. Segundo, dos huevos.", wasn't recognized after M18 and now is.

## Checks

| Check | Result |
|---|---|
| List tests: 6 groups, including 6 ways to end a list and 3 cases of numbers said twice | All pass |
| 533 different texts: earlier tests plus every dictation in your history | The same 14 as in M18 became lists, all meant to. Your test "primero que nada, ahí no debe dar ninguna lista" stays a sentence |
| All unit tests | 113 passed |
| Hidden run of the real app, no window | The spoken list came out right. Dictation, snippets, history search, delete and clear all work as before (`app-final.json`). No volume memory was written |
