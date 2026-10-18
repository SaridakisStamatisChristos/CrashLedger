# Contributing to CrashLedger

Work from a fresh branch, keep PRs scoped, and add an independent negative control for any new correctness mechanism. Changes to scenario v1, verdict meanings or evidence v1 require a versioning rationale and backward compatibility plan. Do not weaken test assertions to make CI pass.

Before opening a PR on Unix-like hosts, run:

~~~bash
bash scripts/check.sh
bash scripts/demo.sh
~~~

CI checks Rust format, Clippy, workspace tests and both end-to-end verdicts on Ubuntu. The portable matrix checks build/tests on macOS and Windows; additional platform differences must be documented rather than ignored. Never commit real tokens, user database files, generated artifacts or confidential scenario content. Do not assert universal exactly-once behavior.

No license is currently granted by this repository. Do not add a license or publish a release without the owner's explicit decision.
