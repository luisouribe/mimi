"""Compare the models on punctuation, language detection and first-use wait."""
import json
import statistics
import sys
from pathlib import Path

OUT = Path(sys.argv[1])
MARKS = set(".,?!。、?!¿¡,")

for path in sorted(OUT.glob("*.json")):
    report = json.loads(path.read_text(encoding="utf-8"))
    if "clips" not in report:
        continue
    clips = report["clips"]
    print(f"== {path.stem}: load and warm-up {report['load_seconds']:.1f} s, first clip {clips[0]['elapsed']:.2f} s, "
          f"peak memory {report['peak_memory_mb']:.0f} MB")
    for lang in sorted({clip["lang"] for clip in clips}):
        group = [clip for clip in clips if clip["lang"] == lang]
        punctuated = sum(any(mark in clip["text"] for mark in MARKS) for clip in group)
        ends = sum(clip["text"].rstrip()[-1:] in MARKS for clip in group if clip["text"].strip())
        empty = sum(not clip["text"].strip() for clip in group)
        typical = statistics.median(clip["elapsed"] for clip in group)
        print(f"   {lang}: punctuated {punctuated}/{len(group)}, ends with a mark {ends}/{len(group)}, "
              f"empty {empty}, median wait {typical:.2f} s")
        for clip in group[:2]:
            print(f"      {clip['text'][:90]}")
