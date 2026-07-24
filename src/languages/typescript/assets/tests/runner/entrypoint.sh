#!/bin/sh
set -eu

mkdir -p /assets
ln -sfn /opt/ptip-ffi-tests/node_modules /assets/node_modules

exec "$@"