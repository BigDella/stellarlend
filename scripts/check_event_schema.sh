#!/usr/bin/env bash
# check_event_schema.sh
#
# CI check: verifies that every #[contractevent] struct in the StellarLend
# workspace contains a `timestamp` field and an explicit `topics` attribute.
#
# Exit code 0 = all checks passed
# Exit code 1 = one or more violations found
#
# Usage:
#   ./scripts/check_event_schema.sh [--no-topics]
#
# Options:
#   --no-topics   Skip the topics check (useful during migration phases)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACT_ROOT="$REPO_ROOT/stellar-lend/contracts"

SKIP_TOPICS=false
for arg in "$@"; do
  [[ "$arg" == "--no-topics" ]] && SKIP_TOPICS=true
done

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

violations=0
checked=0

# Collect all .rs files that contain #[contractevent]
mapfile -t event_files < <(grep -rl '#\[contractevent' "$CONTRACT_ROOT" --include="*.rs" 2>/dev/null || true)

if [[ ${#event_files[@]} -eq 0 ]]; then
  echo -e "${YELLOW}No contractevent files found under $CONTRACT_ROOT${NC}"
  exit 0
fi

for file in "${event_files[@]}"; do
  rel="${file#$REPO_ROOT/}"

  # Extract each contractevent block: look for the struct that follows it.
  # Strategy: read line-by-line, track whether we are in a contractevent block,
  # capture lines until the closing brace.

  in_event=0
  has_timestamp=0
  has_topics=0
  struct_name=""
  brace_depth=0

  while IFS= read -r line; do
    # Detect start of a contractevent annotation
    if [[ "$line" =~ ^[[:space:]]*#\[contractevent ]]; then
      in_event=1
      has_timestamp=0
      has_topics=0
      struct_name=""
      brace_depth=0

      # Check if topics are declared on same line
      if [[ "$line" =~ topics ]]; then
        has_topics=1
      fi
      continue
    fi

    if [[ $in_event -eq 1 ]]; then
      # Lines between annotation and struct opener (could be #[derive(…)] etc.)
      if [[ "$line" =~ ^[[:space:]]*pub[[:space:]]+struct[[:space:]]+([A-Za-z_]+) ]]; then
        struct_name="${BASH_REMATCH[1]}"
      fi

      # Count braces to detect end of struct
      open=$(echo "$line" | tr -cd '{' | wc -c)
      close=$(echo "$line" | tr -cd '}' | wc -c)
      brace_depth=$(( brace_depth + open - close ))

      # Check for timestamp field
      if [[ "$line" =~ pub[[:space:]]+timestamp ]]; then
        has_timestamp=1
      fi

      # End of struct
      if [[ $brace_depth -lt 0 || ( $brace_depth -eq 0 && "$line" =~ "}" && -n "$struct_name" ) ]]; then
        if [[ -n "$struct_name" ]]; then
          checked=$(( checked + 1 ))

          fail=0
          if [[ $has_timestamp -eq 0 ]]; then
            echo -e "${RED}FAIL${NC} [$rel] $struct_name: missing 'pub timestamp: u64' field"
            fail=1
          fi
          if [[ "$SKIP_TOPICS" == "false" && $has_topics -eq 0 ]]; then
            echo -e "${YELLOW}WARN${NC} [$rel] $struct_name: no explicit topics attribute (add topics=[\"…\"])"
            # Warn only — do not fail for missing topics to allow gradual adoption.
          fi
          if [[ $fail -eq 1 ]]; then
            violations=$(( violations + 1 ))
          fi
        fi

        in_event=0
        struct_name=""
        brace_depth=0
      fi
    fi
  done < "$file"
done

echo ""
echo "Checked $checked contractevent structs across ${#event_files[@]} file(s)."

if [[ $violations -gt 0 ]]; then
  echo -e "${RED}$violations violation(s) found. Please add the missing 'timestamp' field(s).${NC}"
  exit 1
else
  echo -e "${GREEN}All contractevent structs are schema-compliant.${NC}"
  exit 0
fi
