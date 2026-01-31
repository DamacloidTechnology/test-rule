// tests/integration_tests.rs
//! Integration tests for the fraud rule engine

use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value, Action};

#[test]
fn test_simple_rule_execution() {
    let dsl = r#"
        rule "high_amount" {
            priority: 100,
            if (txn.amount > 1000) {
                setFraudScore(0.8);
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).expect("Failed to compile");
    
    let transaction = Transaction::new()
        .with_field("amount", Value::Float(5000.0));
    
    let profile = UserProfile::new();
    
    let result = engine.execute(transaction, profile);
    
    assert_eq!(result.actions.len(), 1);
    match &result.actions[0] {
        Action::SetFraudScore { score } => assert_eq!(*score, 0.8),
        _ => panic!("Expected SetFraudScore action"),
    }
}

#[test]
fn test_profile_mutation() {
    let dsl = r#"
        rule "update_counters" {
            priority: 100,
            if (true) {
                profile.txn_count = profile.txn_count + 1;
                profile.total_amount = profile.total_amount + txn.amount;
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    let transaction = Transaction::new()
        .with_field("amount", Value::Float(500.0));
    
    let profile = UserProfile::new()
        .with_field("txn_count", Value::Int(10))
        .with_field("total_amount", Value::Float(5000.0));
    
    let result = engine.execute(transaction, profile);
    
    assert_eq!(result.profile.fields.get("txn_count"), Some(&Value::Int(11)));
    assert_eq!(result.profile.fields.get("total_amount"), Some(&Value::Float(5500.0)));
}

#[test]
fn test_if_else_statement() {
    let dsl = r#"
        rule "check_amount" {
            priority: 100,
            if (txn.amount > 1000) {
                setFraudScore(0.9);
            } else {
                setFraudScore(0.1);
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    // Test high amount
    let high_txn = Transaction::new().with_field("amount", Value::Float(5000.0));
    let result = engine.execute(high_txn, UserProfile::new());
    
    match &result.actions[0] {
        Action::SetFraudScore { score } => assert_eq!(*score, 0.9),
        _ => panic!("Expected SetFraudScore"),
    }
    
    // Test low amount
    let low_txn = Transaction::new().with_field("amount", Value::Float(500.0));
    let result = engine.execute(low_txn, UserProfile::new());
    
    match &result.actions[0] {
        Action::SetFraudScore { score } => assert_eq!(*score, 0.1),
        _ => panic!("Expected SetFraudScore"),
    }
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

#[test]
fn test_multiple_rules() {
    let dsl = r#"
        rule "velocity_check" {
            priority: 100,
            if (profile.txn_count_1h > 10) {
                createCase("MEDIUM", "High velocity");
                profile.velocity_flag = true;
            }
        }
        
        rule "amount_check" {
            priority: 90,
            if (txn.amount > 5000) {
                createCase("HIGH", "Large amount");
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    let transaction = Transaction::new()
        .with_field("amount", Value::Float(10000.0));
    
    let profile = UserProfile::new()
        .with_field("txn_count_1h", Value::Int(15));
    
    let result = engine.execute(transaction, profile);
    
    // Should have 2 createCase actions
    let case_actions: Vec<_> = result.actions.iter()
        .filter(|a| matches!(a, Action::CreateCase { .. }))
        .collect();
    
    assert_eq!(case_actions.len(), 2);
    assert_eq!(result.profile.fields.get("velocity_flag"), Some(&Value::Bool(true)));
}

#[test]
fn test_complex_conditions() {
    let dsl = r#"
        rule "complex" {
            priority: 100,
            if ((txn.amount > 1000 && profile.risk_score > 0.5) || 
                (txn.country != profile.home_country)) {
                setFraudScore(0.8);
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    // Test first condition
    let txn1 = Transaction::new()
        .with_field("amount", Value::Float(2000.0))
        .with_field("country", Value::String("US".to_string()));
    
    let profile1 = UserProfile::new()
        .with_field("risk_score", Value::Float(0.7))
        .with_field("home_country", Value::String("US".to_string()));
    
    let result1 = engine.execute(txn1, profile1);
    assert_eq!(result1.actions.len(), 1);
    
    // Test second condition
    let txn2 = Transaction::new()
        .with_field("amount", Value::Float(100.0))
        .with_field("country", Value::String("UK".to_string()));
    
    let profile2 = UserProfile::new()
        .with_field("risk_score", Value::Float(0.1))
        .with_field("home_country", Value::String("US".to_string()));
    
    let result2 = engine.execute(txn2, profile2);
    assert_eq!(result2.actions.len(), 1);
}

#[test]
fn test_global_functions() {
    let dsl = r#"
        function updateCounters(profile, txn) {
            profile.txn_count = profile.txn_count + 1;
            profile.total_amount = profile.total_amount + txn.amount;
        }
        
        rule "main" {
            priority: 100,
            if (true) {
                updateCounters(profile, txn);
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    let transaction = Transaction::new()
        .with_field("amount", Value::Float(500.0));
    
    let profile = UserProfile::new()
        .with_field("txn_count", Value::Int(5))
        .with_field("total_amount", Value::Float(1000.0));
    
    let result = engine.execute(transaction, profile);
    
    assert_eq!(result.profile.fields.get("txn_count"), Some(&Value::Int(6)));
    assert_eq!(result.profile.fields.get("total_amount"), Some(&Value::Float(1500.0)));
}

#[test]
fn test_create_case_action() {
    let dsl = r#"
        rule "test" {
            priority: 100,
            if (txn.amount > 1000) {
                createCase("HIGH", "Large transaction");
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    let transaction = Transaction::new()
        .with_field("amount", Value::Float(5000.0));
    
    let result = engine.execute(transaction, UserProfile::new());
    
    assert_eq!(result.actions.len(), 1);
    match &result.actions[0] {
        Action::CreateCase { severity, reason, .. } => {
            assert_eq!(severity, "HIGH");
            assert_eq!(reason, "Large transaction");
        }
        _ => panic!("Expected CreateCase action"),
    }
}

#[test]
fn test_bytecode_serialization() {
    let dsl = r#"
        rule "test" {
            priority: 100,
            if (txn.amount > 1000) {
                setFraudScore(0.8);
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    // Serialize to bytecode
    let bytecode = engine.to_bytecode().unwrap();
    
    // Deserialize back
    let engine2 = RuleEngine::from_bytecode(&bytecode).unwrap();
    
    // Test that it works the same
    let transaction = Transaction::new()
        .with_field("amount", Value::Float(5000.0));
    
    let result = engine2.execute(transaction, UserProfile::new());
    
    assert_eq!(result.actions.len(), 1);
}

#[test]
fn test_rule_priority_ordering() {
    let dsl = r#"
        rule "low_priority" {
            priority: 50,
            if (true) {
                profile.order = "second";
            }
        }
        
        rule "high_priority" {
            priority: 100,
            if (true) {
                profile.order = "first";
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    let result = engine.execute(Transaction::new(), UserProfile::new());
    
    // High priority rule should execute first and set order to "first"
    // Then low priority sets it to "second"
    assert_eq!(
        result.profile.fields.get("order"),
        Some(&Value::String("second".to_string()))
    );
}

#[test]
fn test_disabled_rule() {
    let dsl = r#"
        rule "disabled" {
            priority: 100,
            enabled: false,
            if (true) {
                setFraudScore(0.9);
            }
        }
        
        rule "enabled" {
            priority: 90,
            enabled: true,
            if (true) {
                setFraudScore(0.5);
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    let result = engine.execute(Transaction::new(), UserProfile::new());
    
    // Only the enabled rule should execute
    assert_eq!(result.metadata.executed_rules.len(), 1);
    assert_eq!(result.metadata.skipped_rules.len(), 1);
    
    match &result.actions[0] {
        Action::SetFraudScore { score } => assert_eq!(*score, 0.5),
        _ => panic!("Expected SetFraudScore"),
    }
}

#[test]
fn test_performance_500_rules() {
    use std::time::Instant;
    
    // Generate 500 simple rules
    let mut dsl = String::new();
    for i in 0..500 {
        dsl.push_str(&format!(
            r#"
            rule "rule_{}" {{
                priority: {},
                if (txn.amount > {}) {{
                    profile.rule_{}_executed = true;
                }}
            }}
            "#,
            i, 1000 - i, i * 10, i
        ));
    }
    
    let engine = RuleEngine::from_dsl(&dsl).unwrap();
    
    let transaction = Transaction::new()
        .with_field("amount", Value::Float(5000.0));
    
    let profile = UserProfile::new();
    
    // Warm up
    for _ in 0..10 {
        engine.execute(transaction.clone(), profile.clone());
    }
    
    // Measure
    let start = Instant::now();
    let iterations = 100;
    
    for _ in 0..iterations {
        engine.execute(transaction.clone(), profile.clone());
    }
    
    let elapsed = start.elapsed();
    let avg_time = elapsed / iterations;
    
    println!("Average execution time for 500 rules: {:?}", avg_time);
    
    // Should be under 2ms on most systems
    assert!(avg_time.as_millis() < 5, "Execution took {:?}, expected < 5ms", avg_time);
}
