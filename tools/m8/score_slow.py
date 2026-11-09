"""Summarizes the weak-PC runs: how long Mimi takes for each kind of clip."""
import re
import statistics
import sys
from pathlib import Path

folder = Path(sys.argv[1])
print("| Run | Clips finished | Typical wait | Slowest | Typical wait per 10 s of speech |")
print("|---|---|---|---|---|")
for log in sorted(folder.glob("slow-*.log")):
    if log.name.endswith(".err.log"):
        continue
    rows = re.findall(r"([\d.]+)s of speech, cleanup \d+ ms, total ([\d.]+)s", log.read_text(encoding="utf-8", errors="replace"))
    if not rows:
        print(f"| {log.stem[5:]} | 0 | | | |")
        continue
    # The first clip includes warming up, so it's left out of the typical numbers.
    timed = [(float(speech), float(total)) for speech, total in rows[1:]] or [(float(rows[0][0]), float(rows[0][1]))]
    totals = [total for _, total in timed]
    per_ten = [total / speech * 10 for speech, total in timed if speech > 0]
    print(f"| {log.stem[5:]} | {len(rows)} | {statistics.median(totals):.2f} s | {max(totals):.2f} s | {statistics.median(per_ten):.2f} s |")
