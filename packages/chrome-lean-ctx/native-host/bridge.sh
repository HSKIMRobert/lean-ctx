#!/bin/bash
# Wrapper that launches the Python bridge
exec python3 "$(dirname "$0")/bridge.py" "$@"
