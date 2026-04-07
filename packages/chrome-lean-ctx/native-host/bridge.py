#!/usr/bin/env python3
"""lean-ctx native messaging bridge for Chrome.

Reads JSON messages from Chrome via stdin (length-prefixed),
compresses text using the local lean-ctx binary,
returns compressed result via stdout.
"""
import json
import struct
import subprocess
import sys
import os

def find_lean_ctx():
    for candidate in [
        os.path.expanduser("~/.cargo/bin/lean-ctx"),
        "/usr/local/bin/lean-ctx",
        "/opt/homebrew/bin/lean-ctx",
    ]:
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate

    import shutil
    found = shutil.which("lean-ctx")
    if found:
        return found

    return None

def read_message():
    raw = sys.stdin.buffer.read(4)
    if len(raw) < 4:
        return None
    length = struct.unpack("I", raw)[0]
    data = sys.stdin.buffer.read(length)
    if len(data) < length:
        return None
    return json.loads(data.decode("utf-8"))

def send_message(obj):
    encoded = json.dumps(obj, ensure_ascii=False).encode("utf-8")
    sys.stdout.buffer.write(struct.pack("I", len(encoded)))
    sys.stdout.buffer.write(encoded)
    sys.stdout.buffer.flush()

def compress(text, binary_path):
    try:
        env = os.environ.copy()
        env["LEAN_CTX_ACTIVE"] = "0"
        env["NO_COLOR"] = "1"
        result = subprocess.run(
            [binary_path, "-c", "cat"],
            input=text,
            capture_output=True,
            text=True,
            timeout=10,
            env=env,
        )
        compressed = result.stdout if result.returncode == 0 else text
    except (subprocess.TimeoutExpired, FileNotFoundError):
        compressed = text

    input_tokens = len(text) // 4
    output_tokens = len(compressed) // 4
    savings = ((input_tokens - output_tokens) / max(input_tokens, 1)) * 100

    return {
        "compressed": compressed,
        "inputTokens": input_tokens,
        "outputTokens": output_tokens,
        "savings": round(savings, 1),
    }

def main():
    binary = find_lean_ctx()

    while True:
        msg = read_message()
        if msg is None:
            break

        action = msg.get("action", "")
        text = msg.get("text", "")

        if action == "compress" and text:
            if binary:
                send_message(compress(text, binary))
            else:
                send_message({"compressed": text, "savings": 0, "error": "lean-ctx not found"})
        elif action == "ping":
            send_message({"status": "ok", "binary": binary or "not found"})
        else:
            send_message({"error": "unknown action"})

if __name__ == "__main__":
    main()
