#!/usr/bin/env bash
# Clean-checkout v0.1.0 internal candidate rehearsal (no GitHub release, no tag).
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# -gt 1 ]]; then
  echo "Usage: bash scripts/rc_preflight.sh [new-output-directory]" >&2
  exit 4
fi
if [[ $# -eq 1 ]]; then
  root="$1"
  mkdir "$root"
else
  root="$(mktemp -d)"
fi
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
  echo "This binary packaging preflight targets Linux x86_64 only" >&2
  exit 4
fi
echo "RC_DIRECTORY: $root"
cargo build --release --locked --workspace
cli="./target/release/crashledger"
fixture="./target/release/crashledger-demo"
test -x "$cli"
test -x "$fixture"
"$cli" --version | tee "$root/version.txt"
grep -qx 'crashledger 0.1.0' "$root/version.txt"
"$cli" validate scenarios/lost-ack.yaml >"$root/scenario-validation.txt" 2>&1
grep -q 'VALID: lost-ack-order-retry' "$root/scenario-validation.txt"
sed 's/schema_version: 1/schema_version: 2/' scenarios/lost-ack.yaml >"$root/unsupported.yaml"
set +e
"$cli" validate "$root/unsupported.yaml" >"$root/schema-rejection.txt" 2>&1
schema_code=$?
set -e
test "$schema_code" -eq 4
grep -q 'unsupported schema_version' "$root/schema-rejection.txt"
mkdir "$root/samples"
set +e
"$cli" demo --mode broken --artifacts-dir "$root/samples" >"$root/broken.txt" 2>&1
broken_code=$?
set -e
test "$broken_code" -eq 2
grep -q 'Verdict: FAIL' "$root/broken.txt"
grep -q 'orders_for_key = 2; expected = 1' "$root/broken.txt"
"$cli" demo --mode fixed --artifacts-dir "$root/samples" >"$root/fixed.txt" 2>&1
grep -q 'Verdict: PASS' "$root/fixed.txt"
grep -q 'orders_for_key = 1; expected = 1' "$root/fixed.txt"
broken_dir="$(sed -n 's/^Artifacts: //p' "$root/broken.txt" | tail -1)"
fixed_dir="$(sed -n 's/^Artifacts: //p' "$root/fixed.txt" | tail -1)"
test -d "$broken_dir" && test -d "$fixed_dir"
"$cli" report "$broken_dir" --format json >"$root/broken.report.json"
"$cli" report "$fixed_dir" --format json >"$root/fixed.report.json"
python3 scripts/verify_bundle.py "$broken_dir" --expect-mode broken --expect-verdict FAIL --expect-count 2
python3 scripts/verify_bundle.py "$fixed_dir" --expect-mode fixed --expect-verdict PASS --expect-count 1
python3 scripts/audit_dependencies.py "$root/dependency-audit.json"
# Packaging test: both binaries are present and can be unpacked. No upload/publish.
archive="$root/crashledger-v0.1.0-rc-linux-x86_64.tar.gz"
tar -czf "$archive" -C target/release crashledger crashledger-demo
tar -tf "$archive" | grep -qx crashledger
tar -tf "$archive" | grep -qx crashledger-demo
sha256sum "$archive" >"$root/SHA256SUMS"
echo "RC PREFLIGHT VERIFIED: negative FAIL/2, positive PASS/1, "
echo "offline reports and independent SHA-256 evidence readback"
echo "PRIVATE candidate artifacts: $root"
echo "PUBLIC RELEASE NOT AUTHORIZED: project license selection still required"
