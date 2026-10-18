use std::path::Path;
use std::process::{Command, Output};

fn demo(mode: &str, root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_crashledger"))
        .args(["demo", "--mode", mode, "--artifacts-dir"])
        .arg(root)
        .output()
        .expect("launch demo")
}

fn report(path: &Path, format: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_crashledger"))
        .arg("report")
        .arg(path)
        .args(["--format", format])
        .output()
        .expect("launch offline report")
}

#[test]
fn offline_text_and_json_reports_reconstruct_both_modes() {
    for (mode, run_exit, verdict, count) in [
        ("broken", 2, "FAIL", 2),
        ("fixed", 0, "PASS", 1),
    ] {
        let root = tempfile::tempdir().unwrap();
        let output = demo(mode, root.path());
        assert_eq!(
            output.status.code(), Some(run_exit),
            "demo failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let dirs: Vec<_> = std::fs::read_dir(root.path()).unwrap().collect();
        assert_eq!(dirs.len(), 1);
        let path = dirs[0].as_ref().unwrap().path();
        for required in ["scenario.snapshot.yaml", "run.json", "events.jsonl", "observation.json", "verdict.json", "manifest.sha256"] {
            assert!(path.join(required).is_file(), "missing {required}");
        }
        let text = report(&path, "text");
        assert_eq!(text.status.code(), Some(0), "text report stderr={}", String::from_utf8_lossy(&text.stderr));
        let rendered = String::from_utf8_lossy(&text.stdout);
        assert!(rendered.contains("Evidence: VERIFIED"));
        assert!(rendered.contains(&format!("Verdict: {verdict}")));
        assert!(rendered.contains(&format!("Observed orders: {count}")));
        let json = report(&path, "json");
        assert_eq!(json.status.code(), Some(0));
        let parsed: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
        assert_eq!(parsed["verdict"], verdict);
        assert_eq!(parsed["observed"], count);
        assert_eq!(parsed["manifest_verified"], true);
        assert_eq!(parsed["retry_occurred"], true);
    }
}

#[test]
fn tampered_bundle_is_rejected_by_cli() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(demo("fixed", root.path()).status.code(), Some(0));
    let path = std::fs::read_dir(root.path()).unwrap().next().unwrap().unwrap().path();
    std::fs::write(path.join("events.jsonl"), b"tamper").unwrap();
    let output = report(&path, "json");
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("report rejected"));
}
