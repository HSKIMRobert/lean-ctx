#!/usr/bin/env bash
# Native messaging bridge: reads JSON from Chrome, passes to lean-ctx, returns result

LEAN_CTX=$(command -v lean-ctx 2>/dev/null || echo "$HOME/.cargo/bin/lean-ctx")

read_message() {
  local length_bytes
  IFS= read -r -n 4 length_bytes
  if [[ -z "$length_bytes" ]]; then
    exit 0
  fi
  local length
  length=$(printf '%d' "'${length_bytes:0:1}")
  length=$((length + $(printf '%d' "'${length_bytes:1:1}") * 256))
  length=$((length + $(printf '%d' "'${length_bytes:2:1}") * 65536))
  length=$((length + $(printf '%d' "'${length_bytes:3:1}") * 16777216))
  local message
  IFS= read -r -n "$length" message
  echo "$message"
}

send_message() {
  local message="$1"
  local length=${#message}
  printf "\\x$(printf '%02x' $((length & 0xFF)))"
  printf "\\x$(printf '%02x' $(((length >> 8) & 0xFF)))"
  printf "\\x$(printf '%02x' $(((length >> 16) & 0xFF)))"
  printf "\\x$(printf '%02x' $(((length >> 24) & 0xFF)))"
  printf '%s' "$message"
}

while true; do
  msg=$(read_message)
  if [[ -z "$msg" ]]; then
    exit 0
  fi

  action=$(echo "$msg" | python3 -c "import sys,json;print(json.load(sys.stdin).get('action',''))" 2>/dev/null || echo "")
  text=$(echo "$msg" | python3 -c "import sys,json;print(json.load(sys.stdin).get('text',''))" 2>/dev/null || echo "")

  if [[ "$action" == "compress" && -n "$text" ]]; then
    compressed=$(echo "$text" | LEAN_CTX_ACTIVE=0 NO_COLOR=1 "$LEAN_CTX" -c cat 2>/dev/null || echo "$text")
    input_tokens=$(( ${#text} / 4 ))
    output_tokens=$(( ${#compressed} / 4 ))
    savings=0
    if [[ $input_tokens -gt 0 ]]; then
      savings=$(( (input_tokens - output_tokens) * 100 / input_tokens ))
    fi
    response="{\"compressed\":$(python3 -c "import json;print(json.dumps('''$compressed'''))" 2>/dev/null || echo "\"$compressed\""),\"inputTokens\":$input_tokens,\"outputTokens\":$output_tokens,\"savings\":$savings}"
    send_message "$response"
  else
    send_message '{"error":"unknown action"}'
  fi
done
