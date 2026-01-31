// examples/bench_500.rs
//! Quick example to benchmark 500 rules with large transactions/profiles

use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value};
use std::time::Instant;

fn generate_transaction(field_count: usize) -> Transaction {
    let mut txn = Transaction::new();

    for i in 0..field_count {
        let key = format!("txn_f{}", i);
        if i % 3 == 0 {
            txn = txn.with_field(key, Value::Int((i as i64) * 10));
        } else {
            txn = txn.with_field(key, Value::Float((i as f64) * 1.5));
        }
    }

    txn
}

fn generate_profile(field_count: usize) -> UserProfile {
    let mut profile = UserProfile::new();

    for i in 0..field_count {
        let key = format!("pf{}", i);
        profile = profile.with_field(key, Value::Int(0));
    }

    profile
}

fn generate_rules(rule_count: usize, txn_fields: usize, profile_fields: usize) -> String {
    let mut dsl = String::new();

    // Number of individual conditions per rule
    const CONDITIONS_PER_RULE: usize = 20;

    for i in 0..rule_count {
        let mut conds = Vec::with_capacity(CONDITIONS_PER_RULE);

        for c in 0..CONDITIONS_PER_RULE {
            // Alternate between txn and profile checks
            let use_txn = c % 2 == 0;
            let idx = if use_txn {
                (i + c) % txn_fields
            } else {
                (i + c) % profile_fields
            };

            // Pick an operator cycle
            let op = match c % 6 {
                0 => ">",
                1 => "<",
                2 => ">=",
                3 => "<=",
                4 => "!=",
                _ => "==",
            };

            // Thresholds/values vary so conditions are not identical
            let val = ((i + c) * 13 % 1000) * 3;

            let frag = if use_txn {
                format!("txn.txn_f{} {} {}", idx, op, val)
            } else {
                format!("profile.pf{} {} {}", idx, op, val)
            };

            conds.push(frag);
        }

        let cond_str = conds.join(" && ");

        let pf = i % profile_fields;
        let priority = 1000 - (i as i32 % 900);

        dsl.push_str(&format!(
            r#"
            rule "rule_{i}" {{
                priority: {priority},
                if ({cond}) {{
                    // rule body: increment a profile counter
                    profile.pf{pf} = profile.pf{pf} + 1;
                }}
            }}
            "#,
            i = i,
            priority = priority,
            cond = cond_str,
            pf = pf
        ));
    }

    dsl
}

fn main() {
    let txn_fields = 200usize;
    let profile_fields = 300usize;
    let rules = 500usize;

    println!("Generating {} rules (txn_fields={}, profile_fields={})", rules, txn_fields, profile_fields);
    let dsl = generate_rules(rules, txn_fields, profile_fields);

    println!("Compiling...");
    let now = Instant::now();
    let engine = RuleEngine::from_dsl(&dsl).expect("Failed to compile rules");
    let compile_time = now.elapsed();
    println!("Compilation took: {:?}", compile_time);

    // Prepare inputs with values that trigger rules
    let mut txn = generate_transaction(txn_fields);
    for i in 0..txn_fields {
        let key = format!("txn_f{}", i);
        txn = txn.with_field(key, Value::Float(10_000.0 + (i as f64)));
    }

    let profile = generate_profile(profile_fields);

    // Warm up
    for _ in 0..10 {
        let _ = engine.execute(txn.clone(), profile.clone());
    }

    // Measure execution over multiple iterations
    let iterations = 100usize;
    let start = Instant::now();
    for _ in 0..iterations {
        let _res = engine.execute(txn.clone(), profile.clone());
    }
    let total = start.elapsed();
    println!("Executed {} iterations in {:?} -> avg {:?} per run", iterations, total, total / iterations as u32);
}
