// src/lib.rs
//! # Fraud Rule Engine
//!
//! A high-performance, stateless rule engine for fraud detection.
//! Designed to process 10,000+ transactions per second with <2ms latency.
//!
//! ## Example
//!
//! ```rust
//! use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value};
//! use ahash::HashMap;
//!
//! let dsl = r#"
//!     rule "high_amount" {
//!         priority: 100,
//!         if (txn.amount > 1000) {
//!             createCase("HIGH", "Large transaction");
//!             setFraudScore(0.8);
//!         }
//!     }
//! "#;
//!
//! let engine = RuleEngine::from_dsl(dsl).unwrap();
//!
//! let mut txn_fields = HashMap::default();
//! txn_fields.insert("amount".to_string(), Value::Float(5000.0));
//! let transaction = Transaction { fields: txn_fields };
//!
//! let profile = UserProfile { fields: HashMap::default() };
//!
//! let result = engine.execute(transaction, profile);
//! assert_eq!(result.actions.len(), 2); // createCase + setFraudScore
//! ```

pub mod actions;
pub mod compiler;
pub mod parser;
pub mod runtime;


use ahash::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub use actions::Action;
pub use compiler::bytecode::Instruction;
pub use runtime::value::Value;

/// Errors that can occur during compilation
#[derive(Error, Debug)]
pub enum CompilationError {
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Compilation error: {0}")]
    CompileError(String),
    
    #[error("Unknown field: {0}")]
    UnknownField(String),
    
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
}

/// Errors during rule execution
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    
    #[error("Stack underflow")]
    StackUnderflow,
    
    #[error("Invalid operation")]
    InvalidOperation,
}

/// Main rule engine instance
#[derive(Clone)]
pub struct RuleEngine {
    compiled_rules: Arc<Vec<CompiledRule>>,
    global_functions: Arc<HashMap<String, CompiledFunction>>,
}

/// A compiled rule ready for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledRule {
    pub id: String,
    pub priority: i32,
    pub enabled: bool,
    pub bytecode: Vec<Instruction>,
}

/// A compiled global function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFunction {
    pub name: String,
    pub params: Vec<String>,
    pub bytecode: Vec<Instruction>,
}

/// Transaction data (immutable input)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            fields: HashMap::default(),
        }
    }
    
    pub fn with_field(mut self, key: impl Into<String>, value: Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

/// User profile data (can be mutated by rules)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,
}

impl UserProfile {
    pub fn new() -> Self {
        Self {
            fields: HashMap::default(),
        }
    }
    
    pub fn with_field(mut self, key: impl Into<String>, value: Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of rule execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Modified profile (with mutations applied)
    pub profile: UserProfile,
    
    /// Modified transaction (if any fields changed)
    pub transaction: Transaction,
    
    /// Actions emitted by rules (caller must execute these)
    pub actions: Vec<Action>,
    
    /// Execution metadata for monitoring/debugging
    pub metadata: ExecutionMetadata,
}

/// Metadata about rule execution
#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    /// Rules that were executed
    pub executed_rules: Vec<String>,
    
    /// Rules that were skipped
    pub skipped_rules: Vec<String>,
    
    /// Execution time per rule
    pub rule_timings: HashMap<String, std::time::Duration>,
    
    /// Total execution time
    pub total_duration: std::time::Duration,
    
    /// Whether execution was short-circuited via return
    pub short_circuited: bool,
}

impl RuleEngine {
    /// Create a new rule engine from DSL source code
    ///
    /// # Example
    ///
    /// ```rust
    /// use fraud_rule_engine::RuleEngine;
    ///
    /// let dsl = r#"
    ///     rule "test" {
    ///         priority: 100,
    ///         if (txn.amount > 1000) {
    ///             setFraudScore(0.8);
    ///         }
    ///     }
    /// "#;
    ///
    /// let engine = RuleEngine::from_dsl(dsl).unwrap();
    /// ```
    pub fn from_dsl(dsl_source: &str) -> Result<Self, CompilationError> {
        let ast = parser::parse(dsl_source)?;
        let (rules, functions) = compiler::compile(ast)?;
        
        Ok(Self {
            compiled_rules: Arc::new(rules),
            global_functions: Arc::new(functions),
        })
    }
    
    /// Load from pre-compiled bytecode (for hot reload)
    pub fn from_bytecode(data: &[u8]) -> Result<Self, CompilationError> {
        let (rules, functions): (Vec<CompiledRule>, Vec<CompiledFunction>) = 
            bincode::deserialize(data)
                .map_err(|e| CompilationError::CompileError(e.to_string()))?;
        
        let mut func_map = HashMap::default();
        for func in functions {
            func_map.insert(func.name.clone(), func);
        }
        
        Ok(Self {
            compiled_rules: Arc::new(rules),
            global_functions: Arc::new(func_map),
        })
    }
    
    /// Serialize to bytecode for storage/hot reload
    pub fn to_bytecode(&self) -> Result<Vec<u8>, CompilationError> {
        let functions: Vec<_> = self.global_functions.values().cloned().collect();
        let data = (self.compiled_rules.as_ref(), functions);
        
        bincode::serialize(&data)
            .map_err(|e| CompilationError::CompileError(e.to_string()))
    }
    
    /// Execute rules against transaction and profile
    ///
    /// This is the HOT PATH - optimized for minimal latency.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value};
    /// use ahash::HashMap;
    ///
    /// let engine = RuleEngine::from_dsl("rule \"test\" { priority: 1, if (true) {} }").unwrap();
    /// let txn = Transaction::new();
    /// let profile = UserProfile::new();
    ///
    /// let result = engine.execute(txn, profile);
    /// ```
    pub fn execute(
        &self,
        transaction: Transaction,
        profile: UserProfile,
    ) -> ExecutionResult {
        let start = std::time::Instant::now();
        
        let mut ctx = runtime::ExecutionContext::new(transaction, profile);
        
        // Execute each enabled rule in priority order
        for rule in self.compiled_rules.iter() {
            if !rule.enabled {
                ctx.metadata.skipped_rules.push(rule.id.clone());
                continue;
            }
            
            let rule_start = std::time::Instant::now();
            
            // Execute rule bytecode
            runtime::vm::VM::execute(&rule.bytecode, &mut ctx, &self.global_functions);
            
            ctx.metadata.executed_rules.push(rule.id.clone());
            ctx.metadata.rule_timings.insert(
                rule.id.clone(),
                rule_start.elapsed(),
            );
            
            // Check for short-circuit
            if ctx.should_return {
                ctx.metadata.short_circuited = true;
                break;
            }
        }
        
        ctx.metadata.total_duration = start.elapsed();
        
        ExecutionResult {
            profile: ctx.profile,
            transaction: ctx.transaction,
            actions: ctx.actions,
            metadata: ctx.metadata,
        }
    }
    
    /// Validate DSL syntax without compiling
    pub fn validate_dsl(dsl_source: &str) -> Result<(), CompilationError> {
        parser::parse(dsl_source)?;
        Ok(())
    }
    
    /// Get metadata about loaded rules
    pub fn get_rules_metadata(&self) -> Vec<RuleMetadata> {
        self.compiled_rules
            .iter()
            .map(|r| RuleMetadata {
                id: r.id.clone(),
                priority: r.priority,
                enabled: r.enabled,
            })
            .collect()
    }
    
    /// Get list of global functions
    pub fn get_functions(&self) -> Vec<String> {
        self.global_functions.keys().cloned().collect()
    }
}

/// Metadata about a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMetadata {
    pub id: String,
    pub priority: i32,
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rule_execution() {
        let dsl = r#"
            rule "test_rule" {
                priority: 100,
                if (txn.amount > 1000) {
                    setFraudScore(0.8);
                }
            }
        "#;
        
        let engine = RuleEngine::from_dsl(dsl).unwrap();
        
        let transaction = Transaction::new()
            .with_field("amount", Value::Float(5000.0));
        
        let profile = UserProfile::new();
        
        let result = engine.execute(transaction, profile);
        
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.metadata.executed_rules.len(), 1);
    }
    
    #[test]
    fn test_profile_mutation() {
        let dsl = r#"
            rule "update_counter" {
                priority: 100,
                if (true) {
                    profile.txn_count = profile.txn_count + 1;
                }
            }
        "#;
        
        let engine = RuleEngine::from_dsl(dsl).unwrap();
        
        let transaction = Transaction::new();
        let profile = UserProfile::new()
            .with_field("txn_count", Value::Int(5));
        
        let result = engine.execute(transaction, profile);
        
        assert_eq!(
            result.profile.fields.get("txn_count"),
            Some(&Value::Int(6))
        );
    }
    
    #[test]
    fn test_short_circuit() {
        let dsl = r#"
            rule "first" {
                priority: 100,
                if (true) {
                    setFraudScore(0.9);
                    return;
                }
            }
            
            rule "second" {
                priority: 90,
                if (true) {
                    setFraudScore(0.1);
                }
            }
        "#;
        
        let engine = RuleEngine::from_dsl(dsl).unwrap();
        
        let result = engine.execute(Transaction::new(), UserProfile::new());
        
        assert_eq!(result.actions.len(), 1);
        assert!(result.metadata.short_circuited);
        assert_eq!(result.metadata.executed_rules.len(), 1);
    }
}
