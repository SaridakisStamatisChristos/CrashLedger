#!/usr/bin/env python3
"""Locked dependency license *declaration* screen, offline-first.
Does not certify legal compliance; unresolved/undeclared licenses are surfaced.
"""
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
DENIED = re.compile(r"\b(?:AGPL|GPL|LGPL|SSPL|EUPL)(?:[-0-9.]|\b)", re.IGNORECASE)


def main():
    if len(sys.argv) != 2:
        print("usage: audit_dependencies.py OUTPUT.json", file=sys.stderr)
        return 4
    output = Path(sys.argv[1])
    cmd = ["cargo", "metadata", "--locked", "--format-version", "1"]
    # Fresh Linux runners may lack registry records for Windows-only crates
    # still listed in Cargo.lock. Attempt offline first; fall back to the
    # locked graph with registry metadata retrieval, never an unlocked solve.
    resolution = "offline"
    try:
        raw = subprocess.check_output(
            cmd + ["--offline"], cwd=ROOT, timeout=120, stderr=subprocess.DEVNULL
        )
    except subprocess.CalledProcessError:
        resolution = "locked-registry-fallback"
        raw = subprocess.check_output(cmd, cwd=ROOT, timeout=120)
    info = json.loads(raw)
    inventory = []
    denied = []
    review = []
    local = []
    for item in sorted(info["packages"], key=lambda p: (p["name"], p["version"])):
        row = {
            "name": item["name"],
            "version": item["version"],
            "license_declaration": item.get("license"),
            "license_file": item.get("license_file"),
            "source": item.get("source"),
        }
        if item["source"] is None:
            local.append(row)
            continue
        inventory.append(row)
        license_string = item.get("license") or ""
        if DENIED.search(license_string):
            denied.append(row)
        if not license_string or ("/" in license_string and "http" not in license_string):
            review.append(row)
    data = {
        "schema_version": 1,
        "scope": "offline Cargo metadata license declarations only",
        "policy": "deny declared GPL/AGPL/LGPL/SSPL/EUPL; flag undeclared or ambiguous declarations",
        "third_party": inventory,
        "first_party_without_granted_license": [x["name"] for x in local],
        "denied": denied,
        "needs_manual_review": review,
        "compliance_certified": False,
        "metadata_resolution": resolution,
    }
    output.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"DEPENDENCY AUDIT: third-party={len(inventory)} blocked={len(denied)} "
          f"needs-review={len(review)}; provenance={resolution} Cargo metadata")
    if denied:
        for row in denied:
            print(f"DENIED declared license: {row['name']}@{row['version']}: "
                  f"{row['license_declaration']}", file=sys.stderr)
        return 4
    if review:
        print("Manual review remains necessary for undeclared/ambiguous license metadata.")
    print("No project license has been chosen or granted; no release/publication authorized.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
