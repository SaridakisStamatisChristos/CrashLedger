//! PR-06 acceptance: 100 isolated broken/fixed cycles. A failed run fails
//! the test and retains all evidence under the reported temporary root.
use crashledger_core::Verdict;
use crashledger_engine::{run_demo, DemoMode};
use crashledger_evidence::read_report;

#[tokio::test(flavor = "multi_thread")]
async fn one_hundred_isolated_broken_fixed_pairs() {
    let root = tempfile::tempdir().unwrap();
    for round in 0..100 {
        for (mode, verdict, count) in [
            (DemoMode::Broken, Verdict::Fail, 2),
            (DemoMode::Fixed, Verdict::Pass, 1),
        ] {
            let run = run_demo(mode, root.path()).await.unwrap_or_else(|error| {
                panic!("round {round}, {mode:?}: {error}, root={:?}", root.path())
            });
            assert_eq!(run.decision.verdict, verdict, "round {round}");
            assert_eq!(run.observation, Some(count), "round {round}");
            assert_eq!(run.fault_count, 1, "round {round}");
            assert_eq!(run.attempts.len(), 2, "round {round}");
            let report = read_report(&run.run_directory).unwrap();
            assert_eq!(report.verdict, verdict, "round {round}");
            assert_eq!(report.observed, Some(count), "round {round}");
            assert!(report.manifest_verified);
        }
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 200);
}
