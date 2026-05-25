#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <target> <crash_file>"
  echo "Example: $0 lending_critical stellar-lend/fuzz/artifacts/lending_critical/crash-*"
  exit 2
fi

TARGET="$1"
CRASH_FILE="$2"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FUZZ_DIR="$ROOT_DIR/stellar-lend/fuzz"

if [[ ! -f "$CRASH_FILE" ]]; then
  echo "Crash file not found: $CRASH_FILE"
  exit 1
fi

SHA="$(sha256sum "$CRASH_FILE" | awk '{print substr($1,1,12)}')"
REGRESSION_DIR="$FUZZ_DIR/corpus/$TARGET"
REPORT_DIR="$FUZZ_DIR/regressions/$TARGET"
REGRESSION_FILE="$REGRESSION_DIR/regression_$SHA"
REPORT_FILE="$REPORT_DIR/regression_$SHA.md"

mkdir -p "$REGRESSION_DIR" "$REPORT_DIR"
cp "$CRASH_FILE" "$REGRESSION_FILE"

cat > "$REPORT_FILE" <<EOF
# Fuzz Regression $SHA

- Target: \`$TARGET\`
- Corpus file: \`stellar-lend/fuzz/corpus/$TARGET/regression_$SHA\`
- Source crash: \`$CRASH_FILE\`

Replay:

\`\`\`bash
cd stellar-lend
cargo +nightly fuzz run $TARGET fuzz/corpus/$TARGET/regression_$SHA -- -runs=1
\`\`\`

Minimize before committing if the file is large:

\`\`\`bash
cd stellar-lend
cargo +nightly fuzz tmin $TARGET fuzz/corpus/$TARGET/regression_$SHA
\`\`\`
EOF

echo "Regression corpus copied to: $REGRESSION_FILE"
echo "Triage report written to: $REPORT_FILE"
