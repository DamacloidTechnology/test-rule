// examples/basic_usage.rs
//! Basic usage example of the fraud rule engine

use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value, Action};

fn main() {
    println!("=== Fraud Rule Engine - Basic Usage ===\n");
    
    // Define rules in DSL
    let dsl = r#"
        // Global function to update profile counters
        function updateVelocityCounters(profile, txn) {
            profile.txn_count_1h = profile.txn_count_1h + 1;
            profile.txn_amount_1h = profile.txn_amount_1h + txn.amount;
        }
        
        // High velocity detection
        rule "high_velocity" {
            priority: 100,
            if (profile.txn_count_1h > 10) {
                createCase("HIGH", "Excessive transaction velocity detected");
                setFraudScore(0.9);
                return; // Short-circuit, don't process more rules
            }
        }
        
        // Amount spike detection
        rule "amount_spike" {
            priority: 90,
            if (txn.amount > profile.avg_amount * 5) {
                createCase("MEDIUM", "Transaction amount significantly higher than average");
                setFraudScore(0.7);
            }
        }
        
        // Country mismatch
        rule "country_mismatch" {
            priority: 80,
            if (txn.country != profile.home_country) {
                if (profile.travel_mode == false) {
                    createCase("MEDIUM", "Transaction from unusual country");
                    sendAuthAdvise("SMS", "fraud_alert");
                    setFraudScore(0.6);
                }
            }
        }
        
        // Update counters (always runs)
        rule "update_profile" {
            priority: 1,
            if (true) {
                updateVelocityCounters(profile, txn);
            }
        }
    "#;
    
    // Compile rules
    println!("Compiling rules...");
    let engine = RuleEngine::from_dsl(dsl).expect("Failed to compile rules");
    println!("✓ Rules compiled successfully\n");
    
    // Example 1: Normal transaction
    println!("Example 1: Normal Transaction");

    let transaction1 = Transaction::new()
        .with_field("amount", Value::Float(500.0))
        .with_field("country", Value::String("US".to_string()));
    
    let profile1 = UserProfile::new()
        .with_field("txn_count_1h", Value::Int(3))
        .with_field("txn_amount_1h", Value::Float(1200.0))
        .with_field("avg_amount", Value::Float(400.0))
        .with_field("home_country", Value::String("US".to_string()))
        .with_field("travel_mode", Value::Bool(false));
    
    let result1 = engine.execute(transaction1, profile1);
    
    println!("Executed rules: {:?}", result1.metadata.executed_rules);
    println!("Actions: {} action(s)", result1.actions.len());
    println!("Profile updated: txn_count_1h = {:?}", 
        result1.profile.fields.get("txn_count_1h"));
    println!();
    
    // Example 2: High velocity fraud
    println!("Example 2: High Velocity Fraud");

    let transaction2 = Transaction::new()
        .with_field("amount", Value::Float(300.0))
        .with_field("country", Value::String("US".to_string()));
    
    let profile2 = UserProfile::new()
        .with_field("txn_count_1h", Value::Int(12)) // HIGH!
        .with_field("txn_amount_1h", Value::Float(5000.0))
        .with_field("avg_amount", Value::Float(400.0))
        .with_field("home_country", Value::String("US".to_string()))
        .with_field("travel_mode", Value::Bool(false));
    
    let result2 = engine.execute(transaction2, profile2);
    
    println!("Executed rules: {:?}", result2.metadata.executed_rules);
    println!("Short-circuited: {}", result2.metadata.short_circuited);
    println!("Actions:");
    for action in &result2.actions {
        match action {
            Action::CreateCase { severity, reason, .. } => {
                println!("  - Create Case: [{}] {}", severity, reason);
            }
            Action::SetFraudScore { score } => {
                println!("  - Set Fraud Score: {}", score);
            }
            _ => {}
        }
    }
    println!();
    
    // Example 3: Foreign country transaction
    println!("Example 3: Foreign Country Transaction");

    let transaction3 = Transaction::new()
        .with_field("amount", Value::Float(500.0))
        .with_field("country", Value::String("UK".to_string())); // Different country!
    
    let profile3 = UserProfile::new()
        .with_field("txn_count_1h", Value::Int(2))
        .with_field("txn_amount_1h", Value::Float(800.0))
        .with_field("avg_amount", Value::Float(400.0))
        .with_field("home_country", Value::String("US".to_string()))
        .with_field("travel_mode", Value::Bool(false));
    
    let result3 = engine.execute(transaction3, profile3);
    
    println!("Executed rules: {:?}", result3.metadata.executed_rules);
    println!("Actions:");
    for action in &result3.actions {
        match action {
            Action::CreateCase { severity, reason, .. } => {
                println!("  - Create Case: [{}] {}", severity, reason);
            }
            Action::SendAuthAdvise { channel, template, .. } => {
                println!("  - Send Auth Advice: {} via {}", template, channel);
            }
            Action::SetFraudScore { score } => {
                println!("  - Set Fraud Score: {}", score);
            }
            _ => {}
        }
    }
    println!();
    
    // Example 4: Amount spike
    println!("Example 4: Amount Spike");

    let transaction4 = Transaction::new()
        .with_field("amount", Value::Float(5000.0)) // 5x average!
        .with_field("country", Value::String("US".to_string()));
    
    let profile4 = UserProfile::new()
        .with_field("txn_count_1h", Value::Int(2))
        .with_field("txn_amount_1h", Value::Float(800.0))
        .with_field("avg_amount", Value::Float(400.0))
        .with_field("home_country", Value::String("US".to_string()))
        .with_field("travel_mode", Value::Bool(false));
    
    let result4 = engine.execute(transaction4, profile4);
    
    println!("Executed rules: {:?}", result4.metadata.executed_rules);
    println!("Actions:");
    for action in &result4.actions {
        match action {
            Action::CreateCase { severity, reason, .. } => {
                println!("  - Create Case: [{}] {}", severity, reason);
            }
            Action::SetFraudScore { score } => {
                println!("  - Set Fraud Score: {}", score);
            }
            _ => {}
        }
    }
    println!();
    
    // Performance metrics
    println!("Performance Metrics");
    for (rule_name, duration) in &result4.metadata.rule_timings {
        println!("  {}: {:?}", rule_name, duration);
    }
    println!("  Total: {:?}", result4.metadata.total_duration);
}
