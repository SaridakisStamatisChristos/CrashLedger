//! Independently launch the real CLI as a process with a fresh artifact root.
use std::process::{Command, Output};

fn run(mode: &str, root: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_crashledger"))
        .args(["demo", "--mode", mode, "--artifacts-dir"])
        .arg(root)
        .output()
        .expect("run CrashLedger CLI")
}
#[test]
fn golden_broken_and_fixed_end_to_end() {
    let root = tempfile::tempdir().expect("isolation");
    let broken = run("broken", root.path());
    let broken_log = String::from_utf8_lossy(&broken.stdout);
    assert_eq!(
        broken.status.code(),
        Some(2),
        "broken must exit FAIL=2: stdout={broken_log}, stderr={}",
        String::from_utf8_lossy(&broken.stderr)
    );
    assert!(broken_log.contains("Fault: suppress_response [fired 1 time(s)"));
    assert!(broken_log.contains("Attempts: 2 (first: transport error; second: HTTP 201)"));
    assert!(broken_log.contains("orders_for_key = 2; expected = 1"));
    assert!(broken_log.contains("Verdict: FAIL"));

    let fixed = run("fixed", root.path());
    let fixed_log = String::from_utf8_lossy(&fixed.stdout);
    assert_eq!(
        fixed.status.code(),
        Some(0),
        "fixed must exit PASS=0: stdout={fixed_log}, stderr={}",
        String::from_utf8_lossy(&fixed.stderr)
    );
    assert!(fixed_log.contains("Fault: suppress_response [fired 1 time(s)"));
    assert!(fixed_log.contains("Attempts: 2 (first: transport error; second: HTTP 200)"));
    assert!(fixed_log.contains("orders_for_key = 1; expected = 1"));
    assert!(fixed_log.contains("Verdict: PASS"));

    // Each mode has its own SQLite database, not inherited state.
    let directories: Vec<_> = std::fs::read_dir(root.path()).unwrap().collect();
    assert_eq!(directories.len(), 2);
    for entry in directories {
        let path = entry.unwrap().path().join("run-data/orders.sqlite");
        assert!(path.is_file(), "isolated fixture database absent: {path:?}");
    }
}

#[test]
fn missing_artifact_directory_permission_is_runner_error() {
    let root = tempfile::tempdir().unwrap();
    let invalid = root.path().join("not-a-directory");
    std::fs::write(&invalid, b"file").unwrap();
    let output = run("fixed", &invalid);
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR:"));
}
