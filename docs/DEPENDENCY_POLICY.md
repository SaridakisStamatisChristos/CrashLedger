# Dependency and project licensing preflight (PR-08)

CrashLedger currently has **no owner-selected source-code license**. No license is presumed or granted. Do not publish a public GitHub Release, tag, downloadable binary or redistribute packages until the owner explicitly selects a license and the required legal/license review is complete.

Run the dependency declaration inventory (offline-first) on a built workspace:

~~~bash
python3 scripts/audit_dependencies.py dependency-audit.json
~~~

The script attempts cargo metadata --locked --offline first, then allows a locked registry-metadata fallback when fresh runners lack target-specific package records; the resolution path is recorded. It inventories third-party packages plus their declared license and license_file metadata. It fails on declared AGPL, GPL, LGPL, SSPL or EUPL under the conservative v0.1 policy, and surfaces undeclared or ambiguous declarations for **manual review**. First-party crates without a license are listed separately. The JSON explicitly sets compliance_certified to false, because metadata is not a substitute for full license texts, notices, component behavior, legal advice or actual distribution compliance.

The private RC preflight checks that no currently declared dependency is prohibited. Even if that screen passes, a manual inventory/notice/license decision remains a public release blocker. Rust bundles sqlite locally via rusqlite/libsqlite3-sys; verify any applicable source and notice obligations before distribution.

Do not claim license clearance based only on a green CI job. No automatic license file, copyright assignment, or public release is created here.
