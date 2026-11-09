"""Compares the text from clips with the chime against the same clips without it.

Both are cleaned with the current rules, using tidy_check, so they're judged the same way.
"""
import json, subprocess, sys, tempfile
from pathlib import Path

proof = Path(sys.argv[1])
tidy_check = r"C:\mimi-build\app\release\tidy_check.exe"

def cleaned(texts):
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False, encoding="utf-8") as file:
        file.write("\n".join(texts))
    lines = subprocess.run([tidy_check, "lines", file.name], capture_output=True, encoding="utf-8").stdout.splitlines()
    Path(file.name).unlink()
    return [json.loads(line.strip()[3:]) for line in lines if line.startswith("  => ")]

rows = []
for model, base in [("parakeet", "proof/M9/este-parakeet.json"), ("turbo", "proof/M9/este-turbo.json")]:
    plain_clips = json.loads(Path(base).read_text(encoding="utf-8"))["clips"]
    plain = dict(zip([Path(c["file"]).stem for c in plain_clips], cleaned([c["text"] for c in plain_clips])))
    mixed_clips = json.loads((proof / f"chime-{model}.json").read_text(encoding="utf-8"))["clips"]
    same = 0
    for clip, text in zip(mixed_clips, cleaned([c["text"] for c in mixed_clips])):
        stem = Path(clip["file"]).stem
        if text == plain[stem.split("-")[0]]:
            same += 1
        else:
            print(f"  {model} {stem}: without chime {plain[stem.split('-')[0]]!r} | with chime {text!r}")
    print(f"{model}: {same} of {len(mixed_clips)} identical")
