#!/usr/bin/env bash
# extract-storage.sh — Parse Soroban contract source files and emit a JSON
# representation of the storage layout.
#
# Usage:
#   ./extract-storage.sh <repo_root>
#
# The script scans stellar-lend/contracts/**/src/*.rs for:
#   1. Storage access calls  (env.storage().{instance,persistent,temporary}().{set,get,has}())
#   2. #[contracttype] struct definitions  (fields define on-chain data layout)
#   3. #[contracttype] enum definitions    (variants are storage key discriminants)
#
# Output: a single JSON object written to stdout.
# Compatible with both macOS (BSD) and Linux (GNU) toolchains.

set -uo pipefail

REPO_ROOT="${1:?Usage: extract-storage.sh <repo_root>}"
CONTRACTS_DIR="$REPO_ROOT/stellar-lend/contracts"

if [ ! -d "$CONTRACTS_DIR" ]; then
  echo '{"contracts":{}}'
  exit 0
fi

# ── Temporary work files ─────────────────────────────────────────────
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

ACCESSES_FILE="$TMP_DIR/accesses.jsonl"
TYPES_FILE="$TMP_DIR/types.jsonl"
KEYS_FILE="$TMP_DIR/keys.jsonl"

touch "$ACCESSES_FILE" "$TYPES_FILE" "$KEYS_FILE"

# ── 1. Extract storage access calls ──────────────────────────────────

extract_accesses() {
  local file="$1"
  local contract_name
  contract_name=$(echo "$file" | sed -n 's|.*/contracts/\([^/]*\)/.*|\1|p')
  local rel_path="${file#"$REPO_ROOT"/}"

  # Collapse file into one line, split on storage calls, filter matches
  local collapsed
  collapsed=$(tr '\n' ' ' < "$file")

  # Write matches to a temp file to avoid pipefail issues when grep finds nothing
  local matches_file="$TMP_DIR/matches_$$"
  echo "$collapsed" \
    | sed 's/env[[:space:]]*\.[[:space:]]*storage/\nSTORAGE_CALL env.storage/g' \
    | grep '^STORAGE_CALL' > "$matches_file" 2>/dev/null || true

  while IFS= read -r call_line; do
    # Extract tier
    local tier=""
    if echo "$call_line" | grep -qE '\.instance[[:space:]]*\(' 2>/dev/null; then
      tier="instance"
    elif echo "$call_line" | grep -qE '\.persistent[[:space:]]*\(' 2>/dev/null; then
      tier="persistent"
    elif echo "$call_line" | grep -qE '\.temporary[[:space:]]*\(' 2>/dev/null; then
      tier="temporary"
    else
      continue
    fi

    # Extract operation
    local op=""
    for candidate in set get has remove; do
      if echo "$call_line" | grep -qE "\.$candidate[[:space:]]*\(" 2>/dev/null; then
        op="$candidate"
        break
      fi
    done
    if [ -z "$op" ]; then
      continue
    fi

    # Extract key: everything after &...up to the next , or )
    local key_raw
    key_raw=$(echo "$call_line" \
      | sed "s/.*\.$op[[:space:]]*([[:space:]]*&[[:space:]]*//" \
      | sed 's/[,)].*//' \
      | sed 's/^[[:space:]]*//' \
      | sed 's/[[:space:]]*$//')

    # Truncate overly long keys
    if [ "${#key_raw}" -gt 120 ]; then
      key_raw="${key_raw:0:120}"
    fi

    # Determine key type
    local key_kind="variable"
    local key_value="$key_raw"
    if echo "$key_raw" | grep -qE '^"[^"]*"$' 2>/dev/null; then
      key_kind="string_literal"
      key_value=$(echo "$key_raw" | tr -d '"')
    elif echo "$key_raw" | grep -qE '::' 2>/dev/null; then
      key_kind="enum_variant"
    fi

    printf '{"contract":"%s","file":"%s","tier":"%s","operation":"%s","key":"%s","key_kind":"%s"}\n' \
      "$contract_name" "$rel_path" "$tier" "$op" "$key_value" "$key_kind"

  done < "$matches_file"

  rm -f "$matches_file"
}

# ── 2. Extract #[contracttype] struct/enum definitions ───────────────

extract_types() {
  local file="$1"
  local contract_name
  contract_name=$(echo "$file" | sed -n 's|.*/contracts/\([^/]*\)/.*|\1|p')
  local rel_path="${file#"$REPO_ROOT"/}"

  local in_contracttype=0
  local kind="" name="" fields="" variant_list=""
  local brace_depth=0
  local capturing=0

  while IFS= read -r line; do
    # Detect #[contracttype] attribute
    if echo "$line" | grep -qE '^[[:space:]]*#\[contracttype\]' 2>/dev/null; then
      in_contracttype=1
      continue
    fi

    # Skip derive/repr/cfg lines between #[contracttype] and struct/enum
    if [ "$in_contracttype" -eq 1 ] && [ "$capturing" -eq 0 ]; then
      if echo "$line" | grep -qE '^[[:space:]]*#\[' 2>/dev/null; then
        continue
      fi

      # Detect struct opening
      if echo "$line" | grep -qE '^[[:space:]]*pub[[:space:]]+struct[[:space:]]+' 2>/dev/null; then
        kind="struct"
        name=$(echo "$line" | sed -n 's/.*pub[[:space:]]*struct[[:space:]]*\([A-Za-z_][A-Za-z0-9_]*\).*/\1/p')
        fields=""
        capturing=1
        brace_depth=0
        local open_count close_count
        open_count=$(echo "$line" | tr -cd '{' | wc -c | tr -d ' ')
        close_count=$(echo "$line" | tr -cd '}' | wc -c | tr -d ' ')
        brace_depth=$((brace_depth + open_count - close_count))
        if [ "$brace_depth" -le 0 ] && [ "$open_count" -gt 0 ]; then
          printf '{"contract":"%s","file":"%s","kind":"struct","name":"%s","fields":[]}\n' \
            "$contract_name" "$rel_path" "$name"
          in_contracttype=0
          capturing=0
        fi
        continue
      fi

      # Detect enum opening
      if echo "$line" | grep -qE '^[[:space:]]*pub[[:space:]]+enum[[:space:]]+' 2>/dev/null; then
        kind="enum"
        name=$(echo "$line" | sed -n 's/.*pub[[:space:]]*enum[[:space:]]*\([A-Za-z_][A-Za-z0-9_]*\).*/\1/p')
        variant_list=""
        capturing=1
        brace_depth=0
        local open_count close_count
        open_count=$(echo "$line" | tr -cd '{' | wc -c | tr -d ' ')
        close_count=$(echo "$line" | tr -cd '}' | wc -c | tr -d ' ')
        brace_depth=$((brace_depth + open_count - close_count))
        continue
      fi

      # Non-attribute, non-struct/enum line — reset
      in_contracttype=0
      continue
    fi

    # Capturing struct fields
    if [ "$capturing" -eq 1 ] && [ "$kind" = "struct" ]; then
      local open_count close_count
      open_count=$(echo "$line" | tr -cd '{' | wc -c | tr -d ' ')
      close_count=$(echo "$line" | tr -cd '}' | wc -c | tr -d ' ')
      brace_depth=$((brace_depth + open_count - close_count))

      if echo "$line" | grep -qE '^[[:space:]]*pub[[:space:]]+[a-z_][a-z0-9_]*[[:space:]]*:' 2>/dev/null; then
        local fname ftype
        fname=$(echo "$line" | sed -n 's/.*pub[[:space:]]*\([a-z_][a-z0-9_]*\)[[:space:]]*:.*/\1/p')
        ftype=$(echo "$line" | sed 's/[^:]*:[[:space:]]*//' | sed 's/,[[:space:]]*$//' | sed 's/^[[:space:]]*//' | sed 's/[[:space:]]*$//')
        if [ -n "$fields" ]; then
          fields="$fields,"
        fi
        fields="$fields{\"name\":\"$fname\",\"type\":\"$ftype\"}"
      fi

      if [ "$brace_depth" -le 0 ]; then
        printf '{"contract":"%s","file":"%s","kind":"struct","name":"%s","fields":[%s]}\n' \
          "$contract_name" "$rel_path" "$name" "$fields"
        in_contracttype=0
        capturing=0
      fi
      continue
    fi

    # Capturing enum variants
    if [ "$capturing" -eq 1 ] && [ "$kind" = "enum" ]; then
      local open_count close_count
      open_count=$(echo "$line" | tr -cd '{' | wc -c | tr -d ' ')
      close_count=$(echo "$line" | tr -cd '}' | wc -c | tr -d ' ')
      brace_depth=$((brace_depth + open_count - close_count))

      if echo "$line" | grep -qE '^[[:space:]]*[A-Z][A-Za-z0-9_]*' 2>/dev/null; then
        local vname vparams
        vname=$(echo "$line" | sed -n 's/^[[:space:]]*\([A-Z][A-Za-z0-9_]*\).*/\1/p')
        vparams=""
        if echo "$line" | grep -qE '\(' 2>/dev/null; then
          vparams=$(echo "$line" | sed -n 's/.*(\([^)]*\)).*/\1/p')
        fi
        if [ -n "$variant_list" ]; then
          variant_list="$variant_list,"
        fi
        if [ -n "$vparams" ]; then
          variant_list="$variant_list{\"name\":\"$vname\",\"params\":\"$vparams\"}"
        else
          variant_list="$variant_list{\"name\":\"$vname\"}"
        fi
      fi

      if [ "$brace_depth" -le 0 ]; then
        printf '{"contract":"%s","file":"%s","kind":"enum","name":"%s","variants":[%s]}\n' \
          "$contract_name" "$rel_path" "$name" "$variant_list"
        in_contracttype=0
        capturing=0
      fi
      continue
    fi

  done < "$file"
}

# ── Scan all Rust source files ───────────────────────────────────────
while IFS= read -r rs_file; do
  basename_file=$(basename "$rs_file")
  case "$basename_file" in
    *_test.rs|*_tests.rs|test_*.rs|tests.rs) continue ;;
  esac

  extract_accesses "$rs_file" >> "$ACCESSES_FILE" || true
  extract_types "$rs_file" >> "$TYPES_FILE" || true
done < <(find "$CONTRACTS_DIR" -name '*.rs' -not -path '*/target/*' | sort)

# ── Separate key enums ───────────────────────────────────────────────
grep '"kind":"enum"' "$TYPES_FILE" 2>/dev/null | grep -iE '"name":"[^"]*[Kk]ey[^"]*"' > "$KEYS_FILE" 2>/dev/null || true

# ── Assemble final JSON ─────────────────────────────────────────────
CONTRACT_NAMES=$(cat "$ACCESSES_FILE" "$TYPES_FILE" 2>/dev/null \
  | grep -oE '"contract":"[^"]*"' \
  | sort -u \
  | sed 's/"contract":"//;s/"//' || true)

if [ -z "$CONTRACT_NAMES" ]; then
  echo '{"contracts":{}}'
  exit 0
fi

echo "{"
echo '  "contracts": {'

first_contract=1
for cname in $CONTRACT_NAMES; do
  if [ "$first_contract" -eq 0 ]; then echo ","; fi
  first_contract=0

  accesses=$(grep "\"contract\":\"$cname\"" "$ACCESSES_FILE" 2>/dev/null | paste -sd ',' - || echo "")
  types=$(grep "\"contract\":\"$cname\"" "$TYPES_FILE" 2>/dev/null | paste -sd ',' - || echo "")
  keys=$(grep "\"contract\":\"$cname\"" "$KEYS_FILE" 2>/dev/null | paste -sd ',' - || echo "")

  printf '    "%s": {\n' "$cname"
  printf '      "accesses": [%s],\n' "$accesses"
  printf '      "types": [%s],\n' "$types"
  printf '      "key_enums": [%s]\n' "$keys"
  printf '    }'
done

echo ""
echo "  }"
echo "}"
