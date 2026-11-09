"""Makes test recordings with Mimi's start chime in them, as if the microphone heard it.

For each M9 clip: the chime loud before the words, the chime soft before the words, and the
chime soft on top of the first words. A speaker heard through a microphone is usually soft.
"""
import json, sys, wave
from pathlib import Path
import numpy as np

source, chime_file, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])

def read(path):
    with wave.open(str(path)) as w:
        data = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(np.float32) / 32768
        return data, w.getframerate()

def write(path, data, rate):
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(rate)
        w.writeframes((np.clip(data, -1, 1) * 32767).astype(np.int16).tobytes())

chime, chime_rate = read(chime_file)
manifest = json.loads((source / "manifest.json").read_text(encoding="utf-8"))
made = []
for clip in manifest:
    speech, rate = read(source / clip["file"])
    times = np.arange(int(len(chime) * rate / chime_rate)) / rate
    tone = np.interp(times, np.arange(len(chime)) / chime_rate, chime)
    versions = {
        "loud-before": np.concatenate([tone, speech]),
        "soft-before": np.concatenate([tone * 0.3, speech]),
        "soft-over": speech + np.pad(tone * 0.3, (0, max(0, len(speech) - len(tone))))[: len(speech)],
    }
    for name, data in versions.items():
        file = f"es/{Path(clip['file']).stem}-{name}.wav"
        write(out / file, data, rate)
        made.append({**clip, "file": file, "seconds": round(len(data) / rate, 2)})
(out / "manifest.json").write_text(json.dumps(made, ensure_ascii=False, indent=1), encoding="utf-8")
print(len(made), "clips")
