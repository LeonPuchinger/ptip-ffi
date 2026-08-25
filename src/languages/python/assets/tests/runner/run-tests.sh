#!/bin/sh
set -eu

export PYTHONPATH=/assets

if [ -n "${TESTCASES:-}" ]; then
  exec python -m unittest discover -s testcases -p '*_test.py'
fi

exec python -m unittest discover -s testcases -p '*_test.py'
