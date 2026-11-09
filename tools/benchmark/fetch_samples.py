"""Download a small set of real human speech recordings for testing.

Each clip is converted to the format the speech models expect: 16 kHz, mono WAV.
These clips are test material only and are never shipped with the app.
"""
import json
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(sys.argv[1])
COUNT = int(sys.argv[2]) if len(sys.argv) > 2 else 25

SETS = {
    "en": ("hf-internal-testing/librispeech_asr_dummy", "clean", "validation", "text"),
    "es": ("facebook/multilingual_librispeech", "spanish", "test", "transcript"),
    "ja": ("japanese-asr/ja_asr.jsut_basic5000", "default", "test", "transcription"),
}


def get_json(url: str, tries: int = 4) -> dict:
    for attempt in range(tries):
        try:
            with urllib.request.urlopen(url, timeout=60) as response:
                return json.load(response)
        except Exception as error:
            if attempt == tries - 1:
                raise
            print(f"retrying after {error}", flush=True)
            time.sleep(3 * (attempt + 1))
    raise RuntimeError("unreachable")


def main() -> None:
    manifest = []
    for lang, (dataset, config, split, text_field) in SETS.items():
        folder = ROOT / lang
        folder.mkdir(parents=True, exist_ok=True)
        query = urllib.parse.urlencode(
            {"dataset": dataset, "config": config, "split": split, "offset": 0, "length": COUNT}
        )
        rows = get_json(f"https://datasets-server.huggingface.co/rows?{query}")["rows"]
        for row in rows:
            index = row["row_idx"]
            data = row["row"]
            source = data["audio"][0]["src"]
            target = folder / f"{index:03d}.wav"
            with tempfile.TemporaryDirectory() as temp:
                raw = Path(temp) / "clip"
                urllib.request.urlretrieve(source, raw)
                subprocess.run(
                    ["ffmpeg", "-y", "-loglevel", "error", "-i", str(raw),
                     "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", str(target)],
                    check=True,
                )
            seconds = (target.stat().st_size - 44) / 32000
            manifest.append({
                "lang": lang,
                "file": str(target.relative_to(ROOT)).replace("\\", "/"),
                "seconds": round(seconds, 2),
                "reference": data[text_field],
                "source": dataset,
            })
        print(f"{lang}: {len(rows)} clips", flush=True)
    (ROOT / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=1), encoding="utf-8"
    )
    print(f"total clips {len(manifest)}, seconds {sum(m['seconds'] for m in manifest):.0f}")


if __name__ == "__main__":
    main()
