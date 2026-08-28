#!/bin/sh
set -eu

runtime_root=/tmp/ptip-ffi-python-runtime
mkdir -p "$runtime_root/ffi"
cp /assets/caller/__init__.py "$runtime_root/ffi/__init__.py"
ln -sfn /assets/bridge.py "$runtime_root/ffi/bridge.py"
ln -sfn /assets/socket.py "$runtime_root/ffi/socket.py"

export PYTHONPATH="$runtime_root:/assets/tests"

if [ -n "${TESTCASES:-}" ]; then
  exec python -m unittest discover -s testcases -p '*_test.py'
fi

exec python -m unittest discover -s testcases -p '*_test.py'
