"""Prints what each model wrote for the M4 test dictations, before and after cleanup."""
import json
import sys
from pathlib import Path

folder = Path(sys.argv[1])
names = ["parakeet", "turbo", "turbo-auto", "small"]
reports = {name: json.loads((folder / f"raw-{name}.json").read_text(encoding="utf-8")) for name in names if (folder / f"raw-{name}.json").exists()}
for index, clip in enumerate(reports["parakeet"]["clips"]):
    print(f"\n{clip['file']}  SAID: {clip['reference']}")
    for name, report in reports.items():
        result = report['clips'][index]
        print(f"  {name:11} {result['text']!r}")
        print(f"  {'  cleaned':11} {result.get('tidied')!r}")
