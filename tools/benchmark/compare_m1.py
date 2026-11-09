"""Check that Mimi's audio cleanup never loses words, especially at the ends of sentences.

Each clip is compared with the untouched recording recognized directly, so this measures
only what the cleanup changed. Accuracy against the true text is in results.md.
"""
import json
import statistics
import sys
import unicodedata
from pathlib import Path

OUT = Path(sys.argv[1])
BASE = {c["file"]: c for c in json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))["clips"]}


def words(text: str) -> list[str]:
    text = unicodedata.normalize("NFKC", text).casefold()
    text = "".join(ch if not unicodedata.category(ch).startswith(("P", "S")) else " " for ch in text)
    return text.split()


def ending_kept(reference: list[str], text: list[str], count: int = 2) -> bool:
    """True when the true final words appear among the last few words heard."""
    return all(word in text[-(count + 3):] for word in reference[-count:])


for name in ("mic-48k-stereo", "mic-44k-mono", "very-quiet-mic", "silence-around-speech", "noisy-room"):
    clips = json.loads((OUT / f"{name}.json").read_text(encoding="utf-8"))["clips"]
    kept = [c for c in clips if ending_kept(words(c["reference"]), words(c["text"]))]
    base_kept = [c for c in clips if ending_kept(words(c["reference"]), words(BASE[c["file"]]["text"]))]
    print(f"{name}: sentence endings kept {len(kept)} of {len(clips)} (untouched recording: {len(base_kept)})")
    base_files = {c["file"] for c in base_kept}
    for clip in clips:
        if clip["file"] in base_files and clip not in kept:
            print(f"  LOST {clip['file']}: ...{clip['reference'][-45:]} | heard ...{clip['text'][-45:]}")

clips = json.loads((OUT / "mic-48k-stereo.json").read_text(encoding="utf-8"))["clips"]
near = [c for c in clips if 8 <= c["seconds"] <= 12]
print(f"real clips of 8 to 12 s: {len(near)}, median {statistics.median(c['elapsed'] for c in near):.2f} s, "
      f"worst {max(c['elapsed'] for c in near):.2f} s")
print(f"cleanup median: {statistics.median(c['cleanup_ms'] for c in clips):.0f} ms")

long_clip = json.loads((OUT / "long-dictation.json").read_text(encoding="utf-8"))
clip = long_clip["clips"][0]
print(f"long dictation: {clip['seconds']:.0f} s of audio in {clip['elapsed']:.1f} s, {clip['chunks']} pieces, "
      f"peak memory {long_clip['peak_memory_mb']:.0f} MB")
