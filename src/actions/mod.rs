// src/actions/mod.rs
//! Actions that can be emitted by rules
//! 
//! Actions are the outputs of rule execution. The rule engine is stateless,
//! so it doesn't execute these actions itself - it just collects them and
//! returns them to the caller for async execution.

use crate::Value;
use ahash::HashMap;
use serde::{Deserialize, Serialize};

/// Actions emitted by rules during execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Create a fraud case for investigation
    CreateCase {
        severity: String,
        reason: String,
        #[serde(default)]
        metadata: HashMap<String, Value>,
    },
    
    /// Add a comment to a case
    CreateComment {
        #[serde(skip_serializing_if = "Option::is_none")]
        case_id: Option<String>,
        comment: String,
    },
    
    /// Send authentication advice to customer
    SendAuthAdvise {
        channel: String,
        template: String,
        #[serde(default)]
        params: HashMap<String, Value>,
    },
    
    /// Set final fraud score
    SetFraudScore {
        score: f64,
    },
    
    /// Set transaction decision
    SetDecision {
        decision: String, // ALLOW, BLOCK, REVIEW
    },
    
    /// Custom action with arbitrary parameters
    Custom {
        action_name: String,
        #[serde(default)]
        params: HashMap<String, Value>,
    },
}

impl Action {
    /// Create a case action
    pub fn create_case(severity: impl Into<String>, reason: impl Into<String>) -> Self {
        Action::CreateCase {
            severity: severity.into(),
            reason: reason.into(),
            metadata: HashMap::default(),
        }
    }
    
    /// Create a case action with metadata
    pub fn create_case_with_metadata(
        severity: impl Into<String>,
        reason: impl Into<String>,
        metadata: HashMap<String, Value>,
    ) -> Self {
        Action::CreateCase {
            severity: severity.into(),
            reason: reason.into(),
            metadata,
        }
    }
    
    /// Create a comment action
    pub fn create_comment(comment: impl Into<String>) -> Self {
        Action::CreateComment {
            case_id: None,
            comment: comment.into(),
        }
    }
    
    /// Send auth advice action
    pub fn send_auth_advise(
        channel: impl Into<String>,
        template: impl Into<String>,
    ) -> Self {
        Action::SendAuthAdvise {
            channel: channel.into(),
            template: template.into(),
            params: HashMap::default(),
        }
    }
    
    /// Set fraud score action
    pub fn set_fraud_score(score: f64) -> Self {
        Action::SetFraudScore { score }
    }
    
    /// Set decision action
    pub fn set_decision(decision: impl Into<String>) -> Self {
        Action::SetDecision {
            decision: decision.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_serialization() {
        let action = Action::create_case("HIGH", "Test reason");
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("create_case"));
        assert!(json.contains("HIGH"));
    }

    #[test]
    fn test_action_deserialization() {
        let json = r#"{"type":"set_fraud_score","score":0.85}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::SetFraudScore { score } => assert_eq!(score, 0.85),
            _ => panic!("Wrong action type"),
        }
    }
}
