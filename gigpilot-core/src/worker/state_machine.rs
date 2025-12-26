use serde::{Deserialize, Serialize};
use std::fmt;

/// Chase state enumeration representing the stages of invoice chasing.
/// 
/// The state machine progresses through these states:
/// - Pending: Invoice is due but not yet overdue
/// - Overdue: Invoice due date has passed
/// - ChasingLevel1: First chase (polite reminder)
/// - ChasingLevel2: Second chase (firm reminder)
/// - Paid: Invoice has been paid (terminal state)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum ChaseState {
    #[sqlx(rename = "pending")]
    Pending,
    
    #[sqlx(rename = "overdue")]
    Overdue,
    
    #[sqlx(rename = "chasing_level_1")]
    ChasingLevel1,
    
    #[sqlx(rename = "chasing_level_2")]
    ChasingLevel2,
    
    #[sqlx(rename = "paid")]
    Paid,
}

impl fmt::Display for ChaseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChaseState::Pending => write!(f, "pending"),
            ChaseState::Overdue => write!(f, "overdue"),
            ChaseState::ChasingLevel1 => write!(f, "chasing_level_1"),
            ChaseState::ChasingLevel2 => write!(f, "chasing_level_2"),
            ChaseState::Paid => write!(f, "paid"),
        }
    }
}

/// Action to take when transitioning between states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseAction {
    /// Send a polite reminder email
    SendPoliteReminder,
    
    /// Send a firm reminder email
    SendFirmReminder,
    
    /// Mark as paid (no action needed)
    MarkAsPaid,
    
    /// No action required
    NoAction,
}

impl fmt::Display for ChaseAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChaseAction::SendPoliteReminder => write!(f, "send_polite_reminder"),
            ChaseAction::SendFirmReminder => write!(f, "send_firm_reminder"),
            ChaseAction::MarkAsPaid => write!(f, "mark_as_paid"),
            ChaseAction::NoAction => write!(f, "no_action"),
        }
    }
}

/// Trait for state transitions in the invoice chasing state machine.
/// 
/// Defines the logic for determining the next state and action
/// based on the current state of an invoice.
pub trait Transition {
    /// Determines the next state and action based on the current state.
    /// 
    /// # Arguments
    /// 
    /// * `current_state` - The current chase state
    /// * `days_overdue` - Number of days the invoice is overdue
    /// 
    /// # Returns
    /// 
    /// Returns a tuple of (next_state, action_to_take).
    fn transition(current_state: ChaseState, days_overdue: i64) -> (ChaseState, ChaseAction);
    
    /// Gets the initial state for a new invoice.
    /// 
    /// # Returns
    /// 
    /// Returns the initial chase state (typically Pending).
    fn initial_state() -> ChaseState {
        ChaseState::Pending
    }
}

/// Default implementation of the Transition trait for invoice chasing.
/// 
/// Implements the state machine logic:
/// - Pending -> Overdue (when due_date passes)
/// - Overdue -> ChasingLevel1 (after 0 days overdue, send polite reminder)
/// - ChasingLevel1 -> ChasingLevel2 (after 7 days, send firm reminder)
/// - Any state -> Paid (if invoice is marked as paid)
pub struct ChaseStateMachine;

impl Transition for ChaseStateMachine {
    fn transition(current_state: ChaseState, days_overdue: i64) -> (ChaseState, ChaseAction) {
        match current_state {
            ChaseState::Pending => {
                if days_overdue > 0 {
                    (ChaseState::Overdue, ChaseAction::SendPoliteReminder)
                } else {
                    (ChaseState::Pending, ChaseAction::NoAction)
                }
            }
            ChaseState::Overdue => {
                // Immediately send polite reminder when becoming overdue
                (ChaseState::ChasingLevel1, ChaseAction::SendPoliteReminder)
            }
            ChaseState::ChasingLevel1 => {
                // After 7 days of first chase, escalate to firm reminder
                if days_overdue >= 7 {
                    (ChaseState::ChasingLevel2, ChaseAction::SendFirmReminder)
                } else {
                    (ChaseState::ChasingLevel1, ChaseAction::NoAction)
                }
            }
            ChaseState::ChasingLevel2 => {
                // Already at maximum chase level, no further action
                (ChaseState::ChasingLevel2, ChaseAction::NoAction)
            }
            ChaseState::Paid => {
                // Terminal state, no transitions
                (ChaseState::Paid, ChaseAction::NoAction)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_to_overdue_transition() {
        let (next_state, action) = ChaseStateMachine::transition(ChaseState::Pending, 1);
        assert_eq!(next_state, ChaseState::Overdue);
        assert_eq!(action, ChaseAction::SendPoliteReminder);
    }

    #[test]
    fn test_overdue_to_chasing_level_1() {
        let (next_state, action) = ChaseStateMachine::transition(ChaseState::Overdue, 1);
        assert_eq!(next_state, ChaseState::ChasingLevel1);
        assert_eq!(action, ChaseAction::SendPoliteReminder);
    }

    #[test]
    fn test_chasing_level_1_to_level_2() {
        let (next_state, action) = ChaseStateMachine::transition(ChaseState::ChasingLevel1, 7);
        assert_eq!(next_state, ChaseState::ChasingLevel2);
        assert_eq!(action, ChaseAction::SendFirmReminder);
    }

    #[test]
    fn test_paid_state_no_transition() {
        let (next_state, action) = ChaseStateMachine::transition(ChaseState::Paid, 100);
        assert_eq!(next_state, ChaseState::Paid);
        assert_eq!(action, ChaseAction::NoAction);
    }
}

