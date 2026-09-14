import random
import sys
import time
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "input" / "python"))
from index import HashMap

try:
    operations = int(sys.argv[1])
except (IndexError, ValueError):
    operations = -1

if operations <= 0:
    print("Please provide a positive integer for the number of operations.", file=sys.stderr)
    raise SystemExit(1)

key_alphabet = "abc"
max_insert_value = 1000

start = time.perf_counter()

map = HashMap[str, int]()

for _ in range(operations):
    key_length = random.randint(1, 2)
    key = "".join(random.choice(key_alphabet) for _ in range(key_length))
    value = random.randrange(max_insert_value)
    if random.random() < 0.5:
        if map.has(key):
            map.get(key)
    else:
        map.set(key, value)

elapsed_ms = f"{(time.perf_counter() - start) * 1000:.3f}"

results_dir = Path(__file__).resolve().parents[2] / "results"
results_file = results_dir / "native_py.csv"
results_dir.mkdir(parents=True, exist_ok=True)
if not results_file.exists():
    results_file.write_text("timestamp,operations,key_alphabet,max_insert_value,elapsed_ms\n")

line = (
    f"{datetime.now().astimezone().isoformat(timespec='milliseconds')},"
    f"{operations},{key_alphabet},{max_insert_value},{elapsed_ms}\n"
)
with results_file.open("a") as file:
    file.write(line)
