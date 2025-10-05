#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

cargo llvm-cov clean --workspace
# Prebuild the binary into llvm-cov target so tests can find it (avoid pipe SIGPIPE by writing JSON to file)
mkdir -p target/llvm-cov-target
export ARTIFACTS_JSON=target/llvm-cov-target/cargo-artifacts.jsonl
cargo build -p entity-cli --bin entity-cli --target-dir target/llvm-cov-target --message-format=json > "$ARTIFACTS_JSON"
BIN_ABS_PATH=$(python3 - "$ARTIFACTS_JSON" <<'PY'
import sys, json
path = sys.argv[1]
exe = None
with open(path, 'r') as f:
  for line in f:
    try:
      j = json.loads(line)
    except Exception:
      continue
    if j.get('reason') == 'compiler-artifact' and j.get('executable'):
      exe = j['executable']
print(exe or '')
PY
)
if [ -z "$BIN_ABS_PATH" ]; then
  echo "Failed to detect entity-cli executable path" >&2
  exit 1
fi
export CARGO_BIN_EXE_entity_cli="$BIN_ABS_PATH"

# Run coverage tests once, then generate reports
cargo llvm-cov --workspace
cargo llvm-cov report --html
cargo llvm-cov report --summary-only
echo "HTML coverage at target/llvm-cov/html/index.html"

