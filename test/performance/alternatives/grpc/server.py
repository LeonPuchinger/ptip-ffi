#!/usr/bin/env python3
import importlib.util
import os
import sys
import uuid
from concurrent import futures
from pathlib import Path

import grpc

import hashmap_pb2
import hashmap_pb2_grpc

root = Path(os.environ["GRPC_INPUT_ROOT"])
input_path = root / "input" / "python" / "index.py"
spec = importlib.util.spec_from_file_location("performance_library", input_path)
library = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = library
spec.loader.exec_module(library)

instances = {}

class HashMapService(hashmap_pb2_grpc.HashMapServiceServicer):
    def Create(self, request, context):
        handle = uuid.uuid4().hex
        instances[handle] = library.HashMap()
        return hashmap_pb2.Handle(id=handle)

    def Set(self, request, context):
        instances[request.handle].set(request.key, request.value)
        return hashmap_pb2.Empty()

    def Get(self, request, context):
        return hashmap_pb2.Value(value=instances[request.handle].get(request.key))

    def Has(self, request, context):
        return hashmap_pb2.BoolValue(value=instances[request.handle].has(request.key))

server = grpc.server(futures.ThreadPoolExecutor(max_workers=8))
hashmap_pb2_grpc.add_HashMapServiceServicer_to_server(HashMapService(), server)
port = int(os.environ.get("FFI_GRPC_PORT", "50051"))
server.add_insecure_port(f"127.0.0.1:{port}")
server.start()
print("ready", flush=True)
server.wait_for_termination()
