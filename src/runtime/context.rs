// src/runtime/context.rs
//! Execution context that maintains state during rule execution

use crate::{Action, ExecutionMetadata, Transaction, UserProfile, Value};
use ahash::HashMap;

/// Mutable context for rule execution
pub struct ExecutionContext {
    /// Transaction being processed (can be modified)
    pub transaction: Transaction,

    /// User profile (can be modified)
    pub profile: UserProfile,

    /// Actions collected during execution
    pub actions: Vec<Action>,

    /// Execution metadata
    pub metadata: ExecutionMetadata,

    /// Whether a return statement was executed
    pub should_return: bool,

    /// Stack for bytecode VM
    pub stack: Vec<Value>,

    /// Local variables
    pub local_vars: HashMap<String, Value>,
}

impl ExecutionContext {
    pub fn new(transaction: Transaction, profile: UserProfile) -> Self {
        Self {
            transaction,
            profile,
            actions: Vec::new(),
            metadata: ExecutionMetadata {
                executed_rules: Vec::new(),
                skipped_rules: Vec::new(),
                rule_timings: HashMap::default(),
                total_duration: std::time::Duration::ZERO,
                short_circuited: false,
            },
            should_return: false,
            stack: Vec::with_capacity(128), // Pre-allocate for performance
            local_vars: HashMap::default(),
        }
    }

    /// Push value onto stack
    #[inline]
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pop value from stack
    #[inline]
    pub fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    /// Peek at top of stack without removing
    #[inline]
    pub fn peek(&self) -> Option<&Value> {
        self.stack.last()
    }

    /// Get profile field value
    #[inline]
    pub fn get_profile_field(&self, field: &str) -> Value {
        self.profile
            .fields
            .get(field)
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// Set profile field value
    #[inline]
    pub fn set_profile_field(&mut self, field: String, value: Value) {
        self.profile.fields.insert(field, value);
    }

    /// Get transaction field value
    #[inline]
    pub fn get_txn_field(&self, field: &str) -> Value {
        self.transaction
            .fields
            .get(field)
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// Set transaction field value
    #[inline]
    pub fn set_txn_field(&mut self, field: String, value: Value) {
        self.transaction.fields.insert(field, value);
    }

    /// Get local variable
    #[inline]
    pub fn get_local(&self, name: &str) -> Value {
        self.local_vars.get(name).cloned().unwrap_or(Value::Null)
    }

    /// Set local variable
    #[inline]
    pub fn set_local(&mut self, name: String, value: Value) {
        self.local_vars.insert(name, value);
    }

    /// Add an action to be executed
    #[inline]
    pub fn add_action(&mut self, action: Action) {
        self.actions.push(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_operations() {
        let mut ctx = ExecutionContext::new(Transaction::new(), UserProfile::new());

        ctx.push(Value::Int(42));
        ctx.push(Value::Bool(true));

        assert_eq!(ctx.pop(), Some(Value::Bool(true)));
        assert_eq!(ctx.pop(), Some(Value::Int(42)));
        assert_eq!(ctx.pop(), None);
    }

    #[test]
    fn test_field_access() {
        let mut ctx = ExecutionContext::new(Transaction::new(), UserProfile::new());

        ctx.set_profile_field("count".to_string(), Value::Int(5));
        assert_eq!(ctx.get_profile_field("count"), Value::Int(5));
        assert_eq!(ctx.get_profile_field("missing"), Value::Null);
    }
}
