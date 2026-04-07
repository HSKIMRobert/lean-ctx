#!/usr/bin/env bash
set -euo pipefail

LEAN_CTX=$(command -v lean-ctx 2>/dev/null || echo "")
if [[ -z "$LEAN_CTX" ]]; then
  echo "Error: lean-ctx not found in PATH"
  echo "Install: cargo install lean-ctx"
  exit 1
fi

HOST_NAME="com.leanctx.bridge"

case "$(uname)" in
  Darwin)
    TARGET_DIR="$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
    ;;
  Linux)
    TARGET_DIR="$HOME/.config/google-chrome/NativeMessagingHosts"
    ;;
  *)
    echo "Unsupported platform: $(uname)"
    exit 1
    ;;
esac

mkdir -p "$TARGET_DIR"

BRIDGE_SCRIPT="$(cd "$(dirname "$0")" && pwd)/bridge.sh"
chmod +x "$BRIDGE_SCRIPT"

cat > "$TARGET_DIR/$HOST_NAME.json" <<MANIFEST
{
  "name": "$HOST_NAME",
  "description": "lean-ctx native messaging bridge",
  "path": "$BRIDGE_SCRIPT",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://EXTENSION_ID_HERE/"
  ]
}
MANIFEST

echo "Native messaging host installed: $TARGET_DIR/$HOST_NAME.json"
echo "Update EXTENSION_ID_HERE with your actual extension ID after loading."
