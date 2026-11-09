"""Prints Mimi's memory over time from the hidden check."""
import csv
import sys

rows = list(csv.DictReader(open(sys.argv[1], encoding="utf-8-sig")))
step = max(1, len(rows) // 30)
print(" ".join(f"{float(row['seconds']):.0f}s:{row['mb']}MB" for row in rows[::step]))
values = [int(row["mb"]) for row in rows]
print(f"highest {max(values)} MB, lowest after loading {min(values[len(values) // 5:])} MB")
