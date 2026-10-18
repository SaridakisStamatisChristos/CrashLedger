# PR-08 — internal v0.1 release-candidate acceptance

| ID | Hard requirement | Evidence path |
| --- | --- | --- |
| RC01 | Clean checkout builds exact Cargo.lock workspace in --release mode | GitHub Actions release-candidate job |
| RC02 | CLI version 0.1.0, valid v1 accepted, schema v2 rejected with exit 4 | rc_preflight.sh |
| RC03 | Fresh broken/fixed release-binary demos yield FAIL/2/exit2 and PASS/1/exit0 | rc_preflight.sh |
| RC04 | Real evidence generated and independently read by both Rust/Python offline verifiers | rc_preflight.sh |
| RC05 | Locally package Linux x86_64 CLI + fixture; verify archive members and SHA-256 digest | rc_preflight.sh |
| RC06 | Inventory third-party declared licenses from locked Cargo metadata, fail when all SPDX choices require excluded copyleft terms; flag ambiguous/manual | audit_dependencies.py + DEPENDENCY_POLICY |
| RC07 | Linux, macOS, Windows prior PR07 gates remain green | GitHub Actions dependent jobs |
| RC08 | Compatibility declaration, draft release notes, limitations and blockers recorded | docs/COMPATIBILITY, RELEASE_CANDIDATE, RELEASE_NOTES |
| RC09 | No public release, tag or arbitrary license assumption without authorization | inspection of PR diff and GitHub release state |

The internal local candidate is verified when the release-candidate CI succeeds. The first-party project is now Apache-2.0 licensed; public binary distribution still depends on a manual third-party license and notices review and a separate release decision. This acceptance gate does not publish a GitHub Release.

No encrypted/signed artifacts, remote agents, Docker deployment or arbitrary crash durability tests are applicable to this v0.1 candidate.

## Licensing chronology

PR-08 initially established an internal candidate before the project license was selected. The owner subsequently chose Apache-2.0, and the open-source productization change added `LICENSE`, `NOTICE`, Cargo license metadata, and a license-inclusive candidate archive. See [dependency policy](DEPENDENCY_POLICY.md) and [release-candidate status](RELEASE_CANDIDATE.md). Third-party notice completeness remains an explicit manual review item.
