#!/bin/bash
set -e
BIN="$1"
shift
if ! getcap "$BIN" 2>/dev/null | grep -q cap_net_admin; then
    sudo setcap cap_net_admin+ep "$BIN"
fi
exec "$BIN" "$@"
