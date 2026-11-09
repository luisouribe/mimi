# M2 proof: dictate into any app

Status on 2026-11-04: approved. You tested it in your real apps and reported that it worked well. The "Not tested yet" list below was written before your check. Your report didn't itemize each row, so these numbers are the only per-item record.

## Your real dictations

From the timing file, which holds no words. Five dictations contained speech: 4 pasted, and 1 stayed in Mimi's window.

| Measure | Median | Range |
|---|---|---|
| Stop to text | 0.21 s | 0.15 to 3.15 s |
| Shortcut to recording ready | 0.34 s | 0.06 to 0.63 s |
| Microphone ready after | 663 ms | 106 to 768 ms |

The 3.15 s dictation had 27.7 s of speech. Total time also includes Mimi's 400 ms wait before restoring the clipboard, which happens after the text is pasted.

The goal of under 0.1 s from shortcut to listening isn't met, because the microphone takes time to open. The indicator shows GET READY immediately to cover this gap.

## Automatic tests

41 code tests pass.

| Area | Covered |
|---|---|
| Shortcut | Hold, tap, Ctrl released first, repeated keys, normal typing, other Ctrl shortcuts, AltGr + Space, Ctrl + Shift + Space, right Ctrl, recovery after a failed recording |
| History | Saving, order, paging, Spanish accents in search, unusual search text, deleting, reopening, Japanese text stored unchanged, 10,000 entries |
| Pasting | App names and which clipboard formats are copied. Actual pasting is not covered |
| Audio cleanup | The tests from M1 |

## Hidden app test

The app ran with no visible window, a temporary history in memory and WAV files instead of a microphone. Pasting is switched off in this mode. Details are in `app-self-test.json`.

| Check | Result |
|---|---|
| English clip, 5.7 s of speech | Text in 0.46 s, saved to history |
| Spanish clip stopped at 8 s | Text in 0.62 s, saved to history |
| Silence | No text and no history entry |
| Stopping when not recording, starting twice, a missing microphone | Refused with a clear message |
| Real history database | Not created by the test |

## Screens

Checked with simulated data in an isolated preview: the main window at 1280 x 720 through every stage, the Copy button, a new entry appearing in the history, and the indicator in every state. All indicator messages fit its 400-pixel window.

## Not tested yet

- Ctrl + Space on your real keyboard, and how it feels
- Pasting into Notepad, Chrome, WhatsApp, Word and other apps
- Your clipboard coming back afterwards, especially images and large contents
- The indicator never taking focus or blocking clicks
- The tray, closing to the tray, and only one copy running
- The 400 ms clipboard wait being long enough in slow apps

## Noted while testing

- In the hidden test, both history entries were labeled "Mimi", which means Windows saw Mimi's process as active. Your check confirms whether real dictations show the right app and whether launching Mimi affects focus.
- The app is now 30.4 MB, plus DirectML.dll at 17.7 MB.
- History search in Japanese only matches from the start of a phrase. M7 needs to address that.
