#!/usr/bin/env python3
"""Fail closed on first-party license metadata, release package and docs drift.

Only Python 3.11+ standard library. Does not claim third-party legal clearance.
"""
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
EXPECTED_URL = "https://github.com/SaridakisStamatisChristos/CrashLedger"


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def main():
    license_text = (ROOT / "LICENSE").read_text(encoding="utf-8")
    require(license_text.startswith("Apache License\nVersion 2.0, January 2004\n"),
            "LICENSE is not the expected Apache 2.0 text")
    require(len(license_text) > 10000, "LICENSE appears truncated")
    require("END OF TERMS AND CONDITIONS" in license_text,
            "LICENSE missing conclusion")
    require("APPENDIX: How to apply the Apache License to your work." in license_text,
            "LICENSE missing Appendix")

    notice = (ROOT / "NOTICE").read_text(encoding="utf-8")
    require("Copyright 2026 Stamatis-Christos Saridakis" in notice,
            "first-party copyright notice missing")
    require("Apache License, Version 2.0" in notice, "NOTICE omits license")

    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    package = workspace["workspace"]["package"]
    require(package["license"] == "Apache-2.0", "workspace missing SPDX license")
    require(package["repository"] == EXPECTED_URL, "workspace wrong repository URL")
    members = workspace["workspace"]["members"]
    require(len(members) >= 8, "workspace members unexpectedly missing")
    for member in members:
        manifest = ROOT / member / "Cargo.toml"
        require(manifest.is_file(), f"missing manifest {manifest}")
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        pkg = data["package"]
        require(pkg.get("license", {}).get("workspace") is True,
                f"{member}: Apache license not inherited")
        require(pkg.get("repository", {}).get("workspace") is True,
                f"{member}: repository URL not inherited")

    essential = [
        "README.md", "CONTRIBUTING.md", "SECURITY.md", "SUPPORT.md",
        "CODE_OF_CONDUCT.md", "THIRD_PARTY_NOTICES.md",
        ".github/PULL_REQUEST_TEMPLATE.md",
        ".github/ISSUE_TEMPLATE/bug_report.yml",
        ".github/ISSUE_TEMPLATE/feature_request.yml",
        ".github/dependabot.yml",
        "docs/DEPENDENCY_POLICY.md", "docs/RELEASE_CANDIDATE.md",
    ]
    for name in essential:
        require((ROOT / name).is_file(), f"missing community/release file: {name}")

    for path in [ROOT / "README.md", ROOT / "CONTRIBUTING.md",
                 ROOT / "SECURITY.md", ROOT / "SUPPORT.md",
                 ROOT / "docs/ARCHITECTURE.md", ROOT / "docs/ROADMAP.md",
                 ROOT / "docs/SCENARIO_V1.md"]:
        text = path.read_text(encoding="utf-8")
        for raw in re.findall(r"(?<!!)\[[^\]]+\]\(([^)]+)\)", text):
            target = raw.split("#", 1)[0].strip()
            if not target or "://" in target or target.startswith("mailto:"):
                continue
            require((path.parent / target).exists(),
                    f"{path.relative_to(ROOT)}: broken local documentation link {raw}")

    # Keep the active user-facing documentation separate from historical
    # implementation ledgers. Historical files and attribution are preserved.
    public_docs = [
        "README.md", "SUPPORT.md", "docs/ARCHITECTURE.md",
        "docs/ROADMAP.md", "docs/SCENARIO_V1.md",
    ]
    obsolete_internal_terms = (
        r"\\bR7\\b",
        r"\\bUCS[- ]Pro\\b",
        r"\\bTrillion Developer\\b",
        r"\\bAI[- ]agent\\b",
        r"\\bAI[- ]generated\\b",
        r"\\bhandoffs?\\b",
    )
    for name in public_docs:
        contents = (ROOT / name).read_text(encoding="utf-8")
        for pattern in obsolete_internal_terms:
            require(not re.search(pattern, contents, flags=re.IGNORECASE),
                    f"{name}: obsolete internal terminology detected")

    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    require("Apache-2.0" in readme, "README lacks Apache-2.0 license")
    require("scripts/demo.sh" in readme, "README lacks reproducible demo")
    require("INCONCLUSIVE" in readme and "FAIL" in readme,
            "README lacks full verdict taxonomy")

    archive_script = (ROOT / "scripts" / "rc_preflight.sh").read_text(encoding="utf-8")
    for filename in ["LICENSE", "NOTICE", "THIRD_PARTY_NOTICES.md", "README.md"]:
        require(filename in archive_script, f"candidate archive lacks {filename}")
    audit = (ROOT / "scripts" / "audit_dependencies.py").read_text(encoding="utf-8")
    require("first_party_license_errors" in audit,
            "dependency audit no longer checks first-party licensing")
    require("compliance_certified" in audit, "dependency audit lost legal limit")

    print(f"PRODUCT HYGIENE: Apache-2.0 text, {len(members)} workspace packages, "
          f"{len(essential)} community/release documents, local links and archive policy PASS")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, AssertionError, KeyError, TypeError) as error:
        print(f"PRODUCT HYGIENE FAILED: {error}", file=sys.stderr)
        sys.exit(4)
