"""Prints what the hidden app check dictated, and whether a snippet appeared when it should."""
import json
import sys
from pathlib import Path

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
address = "Calle Luna 5,\nCiudad de México"
email = "luis@example.com"
counts = {"said": [0, 0], "normal": [0, 0]}
for item in report["dictations"]:
    name = Path(item["clip"]).stem
    text = item["dictation"]["text"]
    inserted = address in text or email in text
    expected = name[0] in "ab"
    kind = "said" if expected else "normal"
    counts[kind][1] += 1
    counts[kind][0] += inserted == expected
    print(("ok   " if inserted == expected else "WRONG"), name, repr(text), round(item["dictation"]["timings"]["tidy_ms"], 3), "ms")
print(f"Phrase said: {counts['said'][0]} of {counts['said'][1]} inserted")
print(f"Similar words only: {counts['normal'][0]} of {counts['normal'][1]} left alone")
print("saved after the test:", report.get("snippets"))
