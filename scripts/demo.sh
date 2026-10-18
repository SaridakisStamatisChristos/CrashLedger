#!/usr/bin/env bash
# Exit 2 in the broken run is EXPECTED.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked -p crashledger-cli -p crashledger-demo
cli="./target/debug/crashledger"
if [[ ! -x "$cli" ]]; then
  echo "Cannot find built CLI at $cli" >&2
  exit 4
fi
root="$(mktemp -d)"
echo "Demo evidence root: $root"
set +e
"$cli" demo --mode broken --artifacts-dir "$root" >"$root/broken.log" 2>&1
broken_code=$?
set -e
cat "$root/broken.log"
if [[ "$broken_code" -ne 2 ]]; then
  echo "Broken control exit=$broken_code, expected 2" >&2
  exit 4
fi
grep -q 'Verdict: FAIL' "$root/broken.log"
grep -q 'orders_for_key = 2; expected = 1' "$root/broken.log"
"$cli" demo --mode fixed --artifacts-dir "$root" >"$root/fixed.log" 2>&1
cat "$root/fixed.log"
grep -q 'Verdict: PASS' "$root/fixed.log"
grep -q 'orders_for_key = 1; expected = 1' "$root/fixed.log"
broken_dir="$(sed -n 's/^Artifacts: //p' "$root/broken.log" | tail -1)"
fixed_dir="$(sed -n 's/^Artifacts: //p' "$root/fixed.log" | tail -1)"
test -d "$broken_dir"
test -d "$fixed_dir"
python3 scripts/verify_bundle.py "$broken_dir" --expect-mode broken --expect-count 2 --expect-verdict FAIL
python3 scripts/verify_bundle.py "$fixed_dir" --expect-mode fixed --expect-count 1 --expect-verdict PASS
"$cli" report "$broken_dir" --format text
"$cli" report "$fixed_dir" --format json
echo "Both independently checked demos passed. Evidence retained: $root"
