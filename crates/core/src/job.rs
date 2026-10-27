//! Job-Zustände der Verarbeitungs-Queue (AKL-M060-01).
//!
//! `uncertain` ist ein Endzustand für die Automatik: Der Ausgang eines
//! Callbacks an Protect ist unbekannt, eine automatische Wiederholung könnte
//! doppelte Ergebnisse erzeugen. Nur ein Mensch darf ihn erneut einreihen.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
    Retryable,
    Uncertain,
    Cancelled,
}

/// Wer einen Übergang auslöst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    Worker,
    Operator,
}

impl JobState {
    pub const ALL: [JobState; 7] = [
        Self::Queued,
        Self::Running,
        Self::Completed,
        Self::Failed,
        Self::Retryable,
        Self::Uncertain,
        Self::Cancelled,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Retryable => "retryable",
            Self::Uncertain => "uncertain",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Uncertain | Self::Cancelled
        )
    }

    pub fn can_transition(self, next: JobState, actor: Actor) -> bool {
        use JobState::*;
        match (actor, self, next) {
            (Actor::Worker, Queued | Retryable, Running) => true,
            (Actor::Worker, Running, Completed | Failed | Retryable | Uncertain) => true,
            (Actor::Operator, Queued | Retryable | Uncertain | Failed, Cancelled) => true,
            // Manuelles Neu-Einreihen nach Prüfung durch einen Menschen.
            (Actor::Operator, Failed | Uncertain, Queued) => true,
            _ => false,
        }
    }
}

impl std::str::FromStr for JobState {
    type Err = crate::device::UnknownValue;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|state| state.as_str() == s)
            .ok_or_else(|| crate::device::UnknownValue(s.to_owned()))
    }
}

/// Exponentieller Backoff: 30 s · 2^(Versuch-1), höchstens 1 h.
pub fn backoff_seconds(attempt: u32) -> u64 {
    let exp = attempt.saturating_sub(1).min(16);
    (30u64 << exp).min(3600)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Actor::*;
    use JobState::*;

    #[test]
    fn worker_lifecycle() {
        assert!(Queued.can_transition(Running, Worker));
        assert!(Running.can_transition(Completed, Worker));
        assert!(Running.can_transition(Retryable, Worker));
        assert!(Retryable.can_transition(Running, Worker));
    }

    #[test]
    fn uncertain_is_never_retried_automatically() {
        assert!(!Uncertain.can_transition(Running, Worker));
        assert!(!Uncertain.can_transition(Queued, Worker));
        assert!(Uncertain.can_transition(Queued, Operator));
        assert!(Uncertain.is_terminal());
    }

    #[test]
    fn workers_cannot_cancel_and_operators_cannot_run() {
        assert!(!Queued.can_transition(Cancelled, Worker));
        assert!(!Queued.can_transition(Running, Operator));
        assert!(
            !Running.can_transition(Cancelled, Operator),
            "laufende Jobs nicht hart abbrechen"
        );
        assert!(!Completed.can_transition(Queued, Operator));
    }

    #[test]
    fn backoff_grows_and_is_capped() {
        assert_eq!(backoff_seconds(1), 30);
        assert_eq!(backoff_seconds(2), 60);
        assert_eq!(backoff_seconds(4), 240);
        assert_eq!(backoff_seconds(10), 3600);
        assert_eq!(backoff_seconds(u32::MAX), 3600);
    }

    #[test]
    fn roundtrip() {
        for s in JobState::ALL {
            assert_eq!(s.as_str().parse::<JobState>().unwrap(), s);
        }
    }
}
