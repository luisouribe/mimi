"""Counts how often a spoken command worked at each pause length, for each model."""
import json
import re
import sys
from pathlib import Path

folder = Path(sys.argv[1])
names = {"parakeet": "Parakeet V3", "turbo": "Whisper Turbo", "turbo-auto": "Whisper Turbo, detect language", "small": "Whisper Small"}
lengths = ["100", "200", "300", "500", "800"]
print("| Model | " + " | ".join(f"{length} ms" for length in lengths) + " |")
print("|---|" + "---|" * len(lengths))
for name, label in names.items():
    report = json.loads((folder / f"raw-{name}.json").read_text(encoding="utf-8"))
    cells = []
    for length in lengths:
        if length == "800":
            clips = [clip for clip in report["clips"] if re.search(r"/(e01|e05|s01|s05|s06)\.wav$", clip["file"])]
        else:
            clips = [clip for clip in report["clips"] if re.search(rf"/[pqr]{length}[es]\.wav$", clip["file"])]
        worked = sum("\n" in clip["tidied"] for clip in clips)
        cells.append(f"{worked} of {len(clips)}")
    print(f"| {label} | " + " | ".join(cells) + " |")
never = [clip for name in names for clip in json.loads((folder / f"raw-{name}.json").read_text(encoding="utf-8"))["clips"] if re.search(r"/(e04|e07|s04|s08)\.wav$", clip["file"])]
print(f"\nSentences that only contain the words: {sum(chr(10) in clip['tidied'] for clip in never)} false commands in {len(never)} results")
