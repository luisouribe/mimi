"""Create versions of the test recordings that imitate real microphones."""
import json
import subprocess
import sys
from pathlib import Path

SOURCE = Path(sys.argv[1])
TARGET = Path(sys.argv[2])
manifest = json.loads((SOURCE / "manifest.json").read_text(encoding="utf-8"))
clips = [clip for clip in manifest if clip["lang"] in ("en", "es")]


def ffmpeg(*arguments: str) -> None:
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", *arguments], check=True)


for name, rate, channels in (("48k-stereo", "48000", "2"), ("44k-mono", "44100", "1")):
    folder = TARGET / name
    for clip in clips:
        output = folder / clip["file"]
        output.parent.mkdir(parents=True, exist_ok=True)
        ffmpeg("-i", str(SOURCE / clip["file"]), "-ar", rate, "-ac", channels, "-c:a", "pcm_s16le", str(output))
    (folder / "manifest.json").write_text(json.dumps(clips, ensure_ascii=False, indent=1), encoding="utf-8")

# One long dictation, made by joining all the English clips.
english = [clip for clip in clips if clip["lang"] == "en"]
long_folder = TARGET / "long"
(long_folder / "en").mkdir(parents=True, exist_ok=True)
playlist = long_folder / "list.txt"
playlist.write_text("".join(f"file '{(SOURCE / c['file']).as_posix()}'\n" for c in english), encoding="utf-8")
ffmpeg("-f", "concat", "-safe", "0", "-i", str(playlist), "-ar", "48000", "-ac", "1", "-c:a", "pcm_s16le",
       str(long_folder / "en" / "joined.wav"))
(long_folder / "manifest.json").write_text(json.dumps([{
    "lang": "en",
    "file": "en/joined.wav",
    "seconds": sum(c["seconds"] for c in english),
    "reference": " ".join(c["reference"] for c in english),
    "source": "joined",
}], ensure_ascii=False), encoding="utf-8")

# Silence, and a room hum with no speech in it.
special = TARGET / "special"
special.mkdir(parents=True, exist_ok=True)
ffmpeg("-f", "lavfi", "-i", "anullsrc=r=48000:cl=mono", "-t", "3", "-c:a", "pcm_s16le", str(special / "silence.wav"))
ffmpeg("-f", "lavfi", "-i", "anoisesrc=r=48000:a=0.0008:c=pink", "-t", "3", "-c:a", "pcm_s16le", str(special / "room-noise.wav"))
ffmpeg("-f", "lavfi", "-i", "anoisesrc=r=48000:a=0.05:c=pink", "-t", "3", "-c:a", "pcm_s16le", str(special / "loud-noise.wav"))
ffmpeg("-f", "lavfi", "-i", "sine=f=120:r=48000", "-t", "3", "-c:a", "pcm_s16le", str(special / "hum.wav"))

# Speech with background noise, about 15 dB quieter than the voice.
folder = TARGET / "noisy-room"
for clip in clips:
    output = folder / clip["file"]
    output.parent.mkdir(parents=True, exist_ok=True)
    ffmpeg("-i", str(SOURCE / clip["file"]), "-f", "lavfi", "-i", "anoisesrc=r=16000:a=0.012:c=pink",
           "-filter_complex", "[0:a][1:a]amix=inputs=2:duration=first:normalize=0", "-ar", "48000", "-ac", "1",
           "-c:a", "pcm_s16le", str(output))
(folder / "manifest.json").write_text(json.dumps(clips, ensure_ascii=False, indent=1), encoding="utf-8")

print("variants ready:", len(clips), "clips")
