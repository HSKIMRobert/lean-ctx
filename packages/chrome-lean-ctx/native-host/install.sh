#!/usr/bin/env bash
set -euo pipefail

EXTENSION_ID="${1:-}"
if [[ -z "$EXTENSION_ID" ]]; then
  echo "Usage: ./install.sh <chrome-extension-id>"
  echo ""
  echo "The extension ID is shown in the lean-ctx popup or at chrome://extensions"
  exit 1
fi

LEAN_CTX=$(command -v lean-ctx 2>/dev/null || echo "$HOME/.cargo/bin/lean-ctx")
if [[ ! -x "$LEAN_CTX" ]]; then
  echo "Error: lean-ctx not found in PATH or ~/.cargo/bin/"
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
  "description": "lean-ctx native messaging bridge for Chrome",
  "path": "$BRIDGE_SCRIPT",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://$EXTENSION_ID/"
  ]
}
MANIFEST

echo "Native messaging host installed successfully."
echo "  Manifest: $TARGET_DIR/$HOST_NAME.json"
echo "  Extension ID: $EXTENSION_ID"
echo "  Bridge: $BRIDGE_SCRIPT"
echo ""
echo "Restart Chrome to activate."
