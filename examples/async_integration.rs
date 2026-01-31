// examples/async_integration.rs
//! Example showing how to integrate the rule engine with async I/O operations

use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value, Action};
use std::time::Duration;
use std::sync::Arc; // Added for thread-safe reference counting

// Simulated async database operations
struct Database;

impl Database {
    async fn get_user_profile(&self, user_id: &str) -> UserProfile {
        // Simulate async database fetch
        tokio::time::sleep(Duration::from_micros(100)).await;

        // Return mock profile
        UserProfile::new()
            .with_field("user_id", Value::String(user_id.to_string()))
            .with_field("txn_count_1h", Value::Int(5))
            .with_field("txn_amount_1h", Value::Float(2000.0))
            .with_field("avg_amount", Value::Float(500.0))
            .with_field("home_country", Value::String("US".to_string()))
            .with_field("travel_mode", Value::Bool(false))
    }

    async fn update_user_profile(&self, profile: UserProfile) {
        // Simulate async database write
        tokio::time::sleep(Duration::from_micros(50)).await;

        println!("  [DB] Profile updated for user: {:?}",
                 profile.fields.get("user_id"));
    }
}

// Simulated action executor
struct ActionExecutor;

impl ActionExecutor {
    async fn execute_actions(&self, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::CreateCase { severity, reason, .. } => {
                    tokio::time::sleep(Duration::from_micros(20)).await;
                    println!("  [ACTION] Created case [{}]: {}", severity, reason);
                }
                Action::SendAuthAdvise { channel, template, .. } => {
                    tokio::time::sleep(Duration::from_micros(30)).await;
                    println!("  [ACTION] Sent {} via {}", template, channel);
                }
                Action::SetFraudScore { score } => {
                    println!("  [ACTION] Set fraud score: {}", score);
                }
                Action::SetDecision { decision } => {
                    println!("  [ACTION] Decision: {}", decision);
                }
                _ => {}
            }
        }
    }
}

// Main fraud detection service
struct FraudDetectionService {
    rule_engine: RuleEngine,
    database: Database,
    action_executor: ActionExecutor,
}

impl FraudDetectionService {
    fn new(rule_engine: RuleEngine) -> Self {
        Self {
            rule_engine,
            database: Database,
            action_executor: ActionExecutor,
        }
    }

    async fn process_transaction(&self, transaction: Transaction, user_id: String) {
        println!("\n=== Processing Transaction for user {} ===", user_id);

        // Step 1: Fetch user profile (async)
        let profile = self.database.get_user_profile(&user_id).await;

        // Step 2: Execute rules (sync)
        let start = std::time::Instant::now();
        let result = self.rule_engine.execute(transaction, profile);
        let elapsed = start.elapsed();

        println!("    ✓ Rules executed in {:?}", elapsed);

        // Step 3: Execute actions (async)
        if !result.actions.is_empty() {
            self.action_executor.execute_actions(result.actions).await;
        }

        // Step 4: Update profile (async)
        self.database.update_user_profile(result.profile).await;
    }
}

#[tokio::main]
async fn main() {
    println!("=== Fraud Rule Engine - Async Integration Example ===\n");

    let dsl = r#"
        rule "high_velocity" {
            priority: 100,
            if (profile.txn_count_1h > 10) {
                createCase("HIGH", "High velocity detected");
                setFraudScore(0.9);
                setDecision("BLOCK");
                return;
            }
        }

        rule "amount_spike" {
            priority: 90,
            if (txn.amount > profile.avg_amount * 5) {
                createCase("MEDIUM", "Amount spike detected");
                sendAuthAdvise("SMS", "amount_verification");
                setFraudScore(0.7);
            }
        }

        rule "update_counters" {
            priority: 1,
            if (true) {
                profile.txn_count_1h = profile.txn_count_1h + 1;
                profile.txn_amount_1h = profile.txn_amount_1h + txn.amount;
            }
        }
    "#;

    let rule_engine = RuleEngine::from_dsl(dsl).expect("Failed to compile rules");

    // Wrap service in Arc to share across tokio tasks
    let service = Arc::new(FraudDetectionService::new(rule_engine));

    println!("Simulating concurrent transaction processing...\n");

    let mut handles = vec![];

    // We define our test cases here
    let test_cases = vec![
        (300.0, "user_001"),
        (5000.0, "user_002"),
        (450.0, "user_003"),
    ];

    for (amount, id) in test_cases {
        let service_clone = Arc::clone(&service);
        let user_id = id.to_string();

        handles.push(tokio::spawn(async move {
            let txn = Transaction::new()
                .with_field("amount", Value::Float(amount))
                .with_field("country", Value::String("US".to_string()));

            service_clone.process_transaction(txn, user_id).await;
        }));
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    println!("\n{}", "-".repeat(50));
    println!("=== All transactions processed ===");

    // Demonstrate Hot Reload
    println!("\n=== Demonstrating Hot Reload ===\n");

    let bytecode = service.rule_engine.to_bytecode().unwrap();
    let reloaded_engine = RuleEngine::from_bytecode(&bytecode).unwrap();
    let new_service = Arc::new(FraudDetectionService::new(reloaded_engine));

    let txn = Transaction::new()
        .with_field("amount", Value::Float(400.0))
        .with_field("country", Value::String("US".to_string()));

    new_service.process_transaction(txn, "user_004".to_string()).await;

    println!("\n=== Hot reload successful ===");
}