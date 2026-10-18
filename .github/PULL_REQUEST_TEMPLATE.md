## Purpose

Describe the narrowly scoped behavior change and associated requirement or issue.

## Evidence

- [ ] `cargo fmt --all -- --check`, Clippy and workspace tests pass (or explain limitation)
- [ ] Existing broken FAIL / fixed PASS negative and positive controls remain correct
- [ ] New functionality has an independent or adversarial falsification test
- [ ] Scenario/evidence formats, exit contracts and docs updated where necessary
- [ ] No secrets, private SQLite files, run artifacts or credentials committed
- [ ] Added dependencies include a reviewed license declaration and notice plan

## Scope and limits

List any unverified OS, transport behavior, durability claim or release impact. Do not assert broad exactly-once guarantees.
