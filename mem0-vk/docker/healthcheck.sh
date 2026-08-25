#!/bin/sh
set -eu

curl --fail --silent --show-error http://127.0.0.1:6333/healthz >/dev/null
redis-cli -h 127.0.0.1 ping | grep -qx PONG
curl --fail --silent --show-error http://127.0.0.1:8001/health >/dev/null
curl --fail --silent --show-error http://127.0.0.1:8000/health >/dev/null
