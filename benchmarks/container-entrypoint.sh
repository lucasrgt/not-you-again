#!/bin/sh
set -eu

mkdir -p /root/.codex
cp /seed/auth.json /root/.codex/auth.json
chmod 0600 /root/.codex/auth.json

exec "$@"
