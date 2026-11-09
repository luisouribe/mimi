# M7 proof: full history search and reuse

Checked on 2026-11-05 on this PC, with no visible window, microphone, clipboard or key presses, then in the real window by you.

**Your check, 2026-11-05:** you went through search, copy and delete and reported "Funcionó bien".

The Home screen can now search your whole history, copy any result and delete a dictation. Older dictations load as you scroll.

## Speed with 10,000 dictations

A temporary database of 10,000 made-up dictations in Spanish, English and Japanese, 3.1 MB on disk. Each action ran 25 times. Searches stop at the newest 200 matches. These times cover the database only, not drawing the screen.

| Action | Matches | Typical | Slowest |
|---|---|---|---|
| Search: reunion | 200 | 0.56 ms | 0.77 ms |
| Search: MAÑANA | 200 | 0.53 ms | 0.58 ms |
| Search: presupuesto cliente | 200 | 0.56 ms | 0.66 ms |
| Search: pres | 200 | 0.48 ms | 0.58 ms |
| Search: meeting budget | 200 | 0.46 ms | 0.70 ms |
| Search: 予約 | 200 | 1.22 ms | 1.50 ms |
| Search: 編集が | 200 | 1.31 ms | 1.47 ms |
| Search with no results | 0 | 0.08 ms | 0.22 ms |
| Open: newest 100 | 100 | 0.05 ms | 0.08 ms |
| Scroll: 100 older, halfway back | 100 | 0.17 ms | 0.19 ms |
| Save a dictation | 1 | 0.55 ms | 0.91 ms |
| Delete a dictation | 1 | 0.53 ms | 3.11 ms |

## How search behaves

- **Accents and capitals:** ignored, so "reunion" finds "reunión" and "MAÑANA" finds "mañana".
- **Several words:** all must appear, in any order.
- **Start of a word:** "pres" finds "presupuesto", but "supuesto" doesn't find it.
- **Japanese:** now finds text anywhere in a dictation. Before, it only matched the start of a phrase.
- **Results:** newest first, up to 200, with a note when there are more.
- **Clearing:** the X or the Esc key.

## Checks

| Check | Result |
|---|---|
| Unit tests | 84 passed |
| Real app, hidden: dictate, find it, find it in capitals, delete it, confirm it's gone | Passed. See `app-final.json` |
| Screens with 250 simulated dictations: scrolling | Loaded 100, 200, then 250, with no repeats, and stopped asking at the end |
| Screens: searching, Esc to clear, delete with confirmation | Passed |

Delete asks first, with **DELETE** and **KEEP**, because it can't be undone.

## Screenshots

Simulated data: `search.png`, `search-none.png`, `delete-confirm.png` and `smallest-home.png`.

## Known limits

- **Word middles:** Spanish and English search only match from the start of a word.
- **New dictations during a search:** a dictation made while a search is open appears after you clear or change the search.
- **No "delete everything" button.**
- **Japanese word count:** a dictation still counts as one word internally. It isn't shown anywhere.

## Not tested yet

- Scrolling more than 100 real dictations, because you have fewer so far
