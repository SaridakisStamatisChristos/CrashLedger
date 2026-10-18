# PR-08 R7 acceptance, internal v0.1 release candidate

| ID | Hard requirement | Evidence path |
| --- | --- | --- |
| RC01 | Clean checkout builds exact Cargo.lock workspace in --release mode | GitHub Actions release-candidate job |
| RC02 | CLI version 0.1.0, valid v1 accepted, schema v2 rejected with exit 4 | rc_preflight.sh |
| RC03 | Fresh broken/fixed release-binary demos yield FAIL/2/exit2 and PASS/1/exit0 | rc_preflight.sh |
| RC04 | Real evidence generated and independently read by both Rust/Python offline verifiers | rc_preflight.sh |
| RC05 | Locally package Linux x86_64 CLI + fixture; verify archive members and SHA-256 digest | rc_preflight.sh |
| RC06 | Inventory third-party declared licenses, fail on specified copyleft declarations; explicitly flag unknown/manual | audit_dependencies.py + DEPENDENCY_POLICY |
| RC07 | Linux, macOS, Windows prior PR07 gates remain green | GitHub Actions dependent jobs |
| RC08 | Compatibility declaration, draft release notes, limitations and blockers recorded | docs/COMPATIBILITY, RELEASE_CANDIDATE, RELEASE_NOTES |
| RC09 | No public release, tag or arbitrary license assumption without authorization | inspection of PR diff and GitHub release state |

Completion is **VERIFIED FOR DECLARED INTERNAL LOCAL CANDIDATE** if CI passes. A public release is CONDITIONAL on the owner license decision and full third-party compliance review, and is not executed by this PR.

No encrypted/signed artifacts, remote agents, Docker deployment or arbitrary crash durability tests are applicable to this v0.1 candidate.
