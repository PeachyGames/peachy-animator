#!/bin/sh
set -eu
cd "$(dirname "$0")"
if [ -x target/debug/peachy-animator ]; then
  exec ./target/debug/peachy-animator "$@"
fi
exec cargo run --locked -p peachy-animator -- "$@"
