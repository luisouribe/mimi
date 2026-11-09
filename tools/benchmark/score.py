"""Turn benchmark reports into a table of speed and accuracy.

Accuracy is word error rate: the share of words the model got wrong, ignoring
punctuation and capital letters. Japanese has no spaces, so it is scored by
character. Lower is better.
"""
import json
import re
import statistics
import sys
import unicodedata
from pathlib import Path


def normalize(text: str, lang: str) -> list[str]:
    text = unicodedata.normalize("NFKC", text).casefold()
    text = "".join(
        char if not unicodedata.category(char).startswith(("P", "S")) else " "
        for char in text
    )
    if lang == "ja":
        return list(re.sub(r"\s+", "", text))
    return text.split()


def edit_distance(left: list[str], right: list[str]) -> int:
    previous = list(range(len(right) + 1))
    for i, left_item in enumerate(left, 1):
        current = [i]
        for j, right_item in enumerate(right, 1):
            current.append(min(
                previous[j] + 1,
                current[j - 1] + 1,
                previous[j - 1] + (left_item != right_item),
            ))
        previous = current
    return previous[-1]


def main() -> None:
    print("| Model | Language | Word errors | Speed vs real time | Typical time for 10 s of speech | Worst time for 10 s of speech | Load time | Peak memory |")
    print("|---|---|---|---|---|---|---|---|")
    for path in sys.argv[1:]:
        report = json.loads(Path(path).read_text(encoding="utf-8"))
        name = Path(path).stem
        languages = sorted({clip["lang"] for clip in report["clips"]})
        for lang in languages:
            clips = [clip for clip in report["clips"] if clip["lang"] == lang]
            errors = words = 0
            for clip in clips:
                reference = normalize(clip["reference"], lang)
                hypothesis = normalize(clip["text"], lang)
                errors += edit_distance(reference, hypothesis)
                words += len(reference)
            audio = sum(clip["seconds"] for clip in clips)
            elapsed = sum(clip["elapsed"] for clip in clips)
            ratios = [clip["elapsed"] / clip["seconds"] for clip in clips]
            print(
                f"| {name} | {lang} | {100 * errors / words:.1f}% | {audio / elapsed:.1f}x "
                f"| {10 * statistics.median(ratios):.2f} s | {10 * max(ratios):.2f} s "
                f"| {report['load_seconds']:.1f} s | {report['peak_memory_mb']:.0f} MB |"
            )


if __name__ == "__main__":
    main()
