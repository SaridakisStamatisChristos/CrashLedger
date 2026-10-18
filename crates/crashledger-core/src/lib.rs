//! Pure, fixture-independent execution types and verdict evaluation.
//! This crate performs no network or database I/O.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
    Error,
}

impl Verdict {
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Pass => 0,
            Self::Fail => 2,
            Self::Inconclusive => 3,
            Self::Error => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Loaded,
    Validated,
    Ready,
    Executing,
    Quiescing,
    Observing,
    Evaluating,
    Complete,
    Inconclusive,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTransition {
    pub from: Phase,
    pub to: Phase,
}

impl std::fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid state transition: {:?} -> {:?}", self.from, self.to)
    }
}

impl std::error::Error for InvalidTransition {}

impl Phase {
    pub fn transition(self, next: Phase) -> Result<Phase, InvalidTransition> {
        let permitted = matches!(
            (self, next),
            (Self::Loaded, Self::Validated)
                | (Self::Validated, Self::Ready)
                | (Self::Ready, Self::Executing)
                | (Self::Executing, Self::Quiescing)
                | (Self::Quiescing, Self::Observing)
                | (Self::Observing, Self::Evaluating)
                | (Self::Evaluating, Self::Complete)
                | (Self::Quiescing, Self::Inconclusive)
                | (Self::Observing, Self::Inconclusive)
                | (Self::Evaluating, Self::Inconclusive)
                | (Self::Loaded, Self::Error)
                | (Self::Validated, Self::Error)
                | (Self::Ready, Self::Error)
                | (Self::Executing, Self::Error)
                | (Self::Quiescing, Self::Error)
                | (Self::Observing, Self::Error)
                | (Self::Evaluating, Self::Error)
        );
        if permitted {
            Ok(next)
        } else {
            Err(InvalidTransition {
                from: self,
                to: next,
            })
        }
    }
}

/// Final evaluation input, assembled by the engine using typed, independently
/// recorded facts. In PR-01 these facts are exercised only by synthetic tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub infrastructure_failed: bool,
    pub matching_faults_fired: u32,
    pub required_matching_faults: u32,
    pub attempts_sent: u32,
    pub required_attempts: u32,
    pub final_http_status: Option<u16>,
    pub independent_observation: Option<i64>,
    pub expected_integer: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub verdict: Verdict,
    pub reason: &'static str,
}

pub fn evaluate(e: &Evaluation) -> Decision {
    use Verdict::*;
    let (verdict, reason) = if e.infrastructure_failed {
        (Error, "runner or transport infrastructure failed")
    } else if e.matching_faults_fired != e.required_matching_faults {
        (Inconclusive, "fault trigger precondition not established")
    } else if e.attempts_sent != e.required_attempts {
        (Inconclusive, "required retry sequence was not observed")
    } else if !matches!(e.final_http_status, Some(200..=299)) {
        (Inconclusive, "no successful final HTTP response")
    } else if let Some(observed) = e.independent_observation {
        if observed == e.expected_integer {
            (Pass, "observed invariant holds for this run")
        } else {
            (Fail, "independent observation violates invariant")
        }
    } else {
        (Inconclusive, "independent observer did not return a scalar")
    };
    Decision { verdict, reason }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Evaluation {
        Evaluation {
            infrastructure_failed: false,
            matching_faults_fired: 1,
            required_matching_faults: 1,
            attempts_sent: 2,
            required_attempts: 2,
            final_http_status: Some(200),
            independent_observation: Some(1),
            expected_integer: 1,
        }
    }

    #[test]
    fn independent_observation_drives_verdict() {
        let mut e = sample();
        assert_eq!(evaluate(&e).verdict, Verdict::Pass);
        for value in [0, 2, i64::MAX] {
            e.independent_observation = Some(value);
            assert_eq!(evaluate(&e).verdict, Verdict::Fail);
        }
        e.independent_observation = None;
        assert_eq!(evaluate(&e).verdict, Verdict::Inconclusive);
    }

    #[test]
    fn missing_preconditions_never_pass() {
        let mut e = sample();
        e.matching_faults_fired = 0;
        assert_eq!(evaluate(&e).verdict, Verdict::Inconclusive);
        e = sample();
        e.attempts_sent = 1;
        assert_eq!(evaluate(&e).verdict, Verdict::Inconclusive);
        e = sample();
        e.final_http_status = Some(500);
        assert_eq!(evaluate(&e).verdict, Verdict::Inconclusive);
        e = sample();
        e.final_http_status = None;
        assert_eq!(evaluate(&e).verdict, Verdict::Inconclusive);
        e = sample();
        e.infrastructure_failed = true;
        assert_eq!(evaluate(&e).verdict, Verdict::Error);
    }

    #[test]
    fn state_transitions_are_restricted() {
        assert_eq!(Phase::Loaded.transition(Phase::Validated), Ok(Phase::Validated));
        assert!(Phase::Loaded.transition(Phase::Complete).is_err());
        assert!(Phase::Complete.transition(Phase::Executing).is_err());
        assert!(Phase::Observing.transition(Phase::Inconclusive).is_ok());
    }

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(Verdict::Pass.exit_code(), 0);
        assert_eq!(Verdict::Fail.exit_code(), 2);
        assert_eq!(Verdict::Inconclusive.exit_code(), 3);
        assert_eq!(Verdict::Error.exit_code(), 4);
    }
}
