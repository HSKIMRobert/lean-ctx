#!/usr/bin/env bash
# Report production Rust modules that exceed the 500 non-comment LOC target.
set -euo pipefail

LIMIT=500

cd "$(dirname "$0")/.."

count_loc() {
  awk '
    {
      line = $0
      active = 1

      while (active) {
        sub(/^[[:space:]]+/, "", line)

        if (in_block_comment) {
          end = index(line, "*/")
          if (!end) {
            active = 0
          } else {
            line = substr(line, end + 2)
            in_block_comment = 0
          }
        } else if (line == "" || line ~ /^\/\//) {
          active = 0
        } else if (line ~ /^\/\*/) {
          end = index(line, "*/")
          if (!end) {
            in_block_comment = 1
            active = 0
          } else {
            line = substr(line, end + 2)
          }
        } else {
          lines++
          active = 0
        }
      }
    }
    END { print lines + 0 }
  ' "$1"
}

violations=0
while IFS= read -r file; do
  case "$file" in
    */mod.rs | */tests.rs | */tests/* | */test_*.rs | */tests_*.rs | */*_test.rs | */*_tests.rs)
      continue
      ;;
  esac

  lines=$(count_loc "$file")
  if ((lines > LIMIT)); then
    echo "FAIL: $file has $lines non-comment LOC (> $LIMIT)"
    ((violations += 1))
  fi
done < <(find rust/src -type f -name '*.rs' | sort)

echo "$violations files exceed $LIMIT LOC limit"
exit $((violations > 0))
