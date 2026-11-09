# M10 proof: chimes and silencing other apps

Built and checked on 2026-11-05. The new installer is at `C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe` (1.44 GB).

## What's new

Two new switches in Settings, both on by default. Screenshots: `settings.png` and `settings-smallest.png`.

- **Chime when I start and stop:** a rising two-note "tu-rún" when Mimi starts listening and a falling one when she stops. Each lasts about a third of a second. Listen to them in `chime-start.wav` and `chime-stop.wav`.
- **Silence other apps while I talk:** music, videos and every other app fade to silence in about 0.16 seconds when you press Ctrl + Space. They fade back in about half a second, just after the stop chime.

How the silencing works:

- It uses each app's own slider in Windows' Volume mixer, so Mimi's chime stays audible and your main volume is never touched.
- If you move an app's slider while dictating, your choice is kept.
- Windows remembers those sliders, so Mimi first saves the levels in `other-apps-volume.json` in her data folder. If she ever closes in the middle of a dictation, the next time she opens she puts them back.
- An app that starts playing after you've pressed the shortcut isn't silenced until the next dictation.

## Checks

| Check | Result |
|---|---|
| Unit tests | 95 passed, up from 89 |
| An app is silenced and then returned to its exact level | Passed, on this test program's own slider, so none of your apps were touched |
| A slider you move while dictating is left alone | Passed |
| Levels left low by a crash come back the next time Mimi opens | Passed |
| Both chimes are valid sound, short, and end without a click | Passed |
| The hidden app run never chimes or silences anything | Passed, no volume file was written |

## The microphone hearing the chime

The start chime plays while the microphone opens, so the microphone may pick it up. To check it doesn't become words, the start chime was mixed into the 12 M9 test clips three ways: loud before the words, soft before the words, and soft on top of the first words. Those 36 clips went through both models. Results are in `chime-parakeet.json` and `chime-turbo.json`.

| Model | Same text as without the chime |
|---|---|
| Parakeet | 36 of 36 |
| Whisper Turbo | 34 of 36. The other 2 came out better, see below |

The chime never became a word. With the chime first, the models sometimes wrote the "este" pause as a period, as in "Pues este. No sé" and "Fui a la. Este. Tienda". The "este" rule now handles that too: "Pues no sé qué decirte." and "Fui a la tienda de la esquina." In 2 clips, Whisper Turbo wrote "Este." instead of "¿Este?", so it's now removed: "Quería preguntarte algo."

All saved texts from earlier tests were run through the updated rule again. None lost a real "este".

## Not checked yet

What you hear, and whether your music fades away and comes back. I can't play sound on your PC or lower your apps without taking over your desktop, so you're the one who checks this.
