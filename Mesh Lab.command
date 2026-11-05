#!/bin/sh
set -eu
TASK_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
exec "$TASK_DIR/Peachy Animator.command" "$TASK_DIR/examples/peach-bot/mesh-bot.peachy"
