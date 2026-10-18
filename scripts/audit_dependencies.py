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


def license_has_permitted_choice(expression):
    """Evaluate simple SPDX AND/OR/WITH precedence conservatively.

    Return True if at least one complete license alternative has no declared
    disallowed copyleft term, False if all alternatives contain one, and
    None if syntax is unsupported (then require manual review).
    """
    tokens = re.findall(r"\(|\)|\bAND\b|\bOR\b|\bWITH\b|[A-Za-z0-9.+:-]+", expression, re.I)
    reconstructed = re.sub(r"\s+", "", "".join(tokens))
    if reconstructed.casefold() != re.sub(r"\s+", "", expression).casefold():
        return None
    index = 0

    def primary():
        nonlocal index
        if index >= len(tokens):
            raise ValueError("missing SPDX term")
        token = tokens[index]
        index += 1
        if token == "(":
            value = disjunction()
            if index >= len(tokens) or tokens[index] != ")":
                raise ValueError("unclosed SPDX grouping")
            index += 1
            return value
        if token.upper() in ("AND", "OR", "WITH", ")"):
            raise ValueError("unexpected SPDX operator")
        value = not DENIED.search(token)
        if index < len(tokens) and tokens[index].upper() == "WITH":
            index += 1
            if index >= len(tokens) or tokens[index] in ("(", ")", "OR", "AND", "WITH"):
                raise ValueError("missing SPDX exception")
            index += 1
        return value

    def conjunction():
        nonlocal index
        value = primary()
        while index < len(tokens) and tokens[index].upper() == "AND":
            index += 1
            value = primary() and value
        return value

    def disjunction():
        nonlocal index
        value = conjunction()
        while index < len(tokens) and tokens[index].upper() == "OR":
            index += 1
            value = conjunction() or value
        return value

    try:
        result = disjunction()
        if index != len(tokens):
            return None
        return result
    except ValueError:
        return None


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
    first_party_issues = []
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
            if item.get("license") != "Apache-2.0":
                first_party_issues.append(row)
            continue
        inventory.append(row)
        license_string = item.get("license") or ""
        eligibility = license_has_permitted_choice(license_string) if license_string else None
        if eligibility is False:
            denied.append(row)
        if eligibility is None or not license_string:
            review.append(row)
    data = {
        "schema_version": 1,
        "scope": "offline Cargo metadata license declarations only",
        "policy": "deny expressions lacking a permitted SPDX alternative; flag undeclared or ambiguous metadata",
        "third_party": inventory,
        "first_party": local,
        "first_party_license_errors": first_party_issues,
        "first_party_without_granted_license": [x["name"] for x in first_party_issues],
        "denied": denied,
        "needs_manual_review": review,
        "compliance_certified": False,
        "metadata_resolution": resolution,
    }
    output.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"DEPENDENCY AUDIT: third-party={len(inventory)} blocked={len(denied)} "
          f"needs-review={len(review)}; provenance={resolution} Cargo metadata")
    if first_party_issues:
        for row in first_party_issues:
            print(f"INVALID first-party license: {row['name']}@{row['version']}: "
                  f"{row['license_declaration']}", file=sys.stderr)
        return 4
    if denied:
        for row in denied:
            print(f"DENIED declared license: {row['name']}@{row['version']}: "
                  f"{row['license_declaration']}", file=sys.stderr)
        return 4
    if review:
        print("Manual review remains necessary for undeclared/ambiguous license metadata.")
    print("First-party license: Apache-2.0. Manual third-party notices review remains required before an official binary release.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
