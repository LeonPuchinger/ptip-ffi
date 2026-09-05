#!/usr/bin/env python3
import csv
import os
import random
import sys
import time
from datetime import datetime
from pathlib import Path

import grpc

import hashmap_pb2
import hashmap_pb2_grpc

operations = int(sys.argv[1])
results_file = Path(sys.argv[2])
port = int(os.environ.get("FFI_GRPC_PORT", "50051"))
channel = grpc.insecure_channel(f"127.0.0.1:{port}")
stub = hashmap_pb2_grpc.HashMapServiceStub(channel)
handle = stub.Create(hashmap_pb2.Empty()).id
key_alphabet = "abc"
max_insert_value = 1000
start = time.perf_counter()
for _ in range(operations):
    key = "".join(random.choice(key_alphabet) for _ in range(random.randint(1, 2)))
    value = random.randrange(max_insert_value)
    if random.random() < 0.5:
        if stub.Has(hashmap_pb2.Key(handle=handle, key=key)).value:
            stub.Get(hashmap_pb2.Key(handle=handle, key=key))
    else:
        stub.Set(hashmap_pb2.Entry(handle=handle, key=key, value=value))
elapsed = f"{(time.perf_counter() - start) * 1000:.3f}"
results_file.parent.mkdir(parents=True, exist_ok=True)
new_file = not results_file.exists()
with results_file.open("a", newline="") as output:
    writer = csv.writer(output)
    if new_file:
        writer.writerow(["timestamp", "operations", "key_alphabet", "max_insert_value", "elapsed_ms"])
    writer.writerow([datetime.now().astimezone().isoformat(timespec="milliseconds"), operations, key_alphabet, max_insert_value, elapsed])
