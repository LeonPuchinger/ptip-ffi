import random
import sys
import time
from datetime import datetime
from pathlib import Path

from caller.index import HashMap

try:
    operations = int(sys.argv[1])
    results_file = Path(sys.argv[2])
except (IndexError, ValueError):
    print("Usage: usage.py OPERATIONS RESULTS_FILE", file=sys.stderr)
    raise SystemExit(1)

if operations <= 0:
    print("Please provide a positive integer for the number of operations.", file=sys.stderr)
    raise SystemExit(1)

key_alphabet = "abc"
max_insert_value = 1000
start = time.perf_counter()
map = HashMap()

for _ in range(operations):
    key = "".join(random.choice(key_alphabet) for _ in range(random.randint(1, 2)))
    value = random.randrange(max_insert_value)
    if random.random() < 0.5:
        if map.has(key):
            map.get(key)
    else:
        map.set(key, value)

elapsed_ms = f"{(time.perf_counter() - start) * 1000:.3f}"
results_file.parent.mkdir(parents=True, exist_ok=True)
if not results_file.exists():
    results_file.write_text("timestamp,operations,key_alphabet,max_insert_value,elapsed_ms\n")

with results_file.open("a") as file:
    file.write(
        f"{datetime.now().astimezone().isoformat(timespec='milliseconds')},"
        f"{operations},{key_alphabet},{max_insert_value},{elapsed_ms}\n"
    )
