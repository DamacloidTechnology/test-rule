// benches/benchmark_large_payloads.rs
//! Benchmark: 500 rules against large transactions/profiles
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value};
use std::time::Instant;

fn generate_transaction(field_count: usize) -> Transaction {
    let mut txn = Transaction::new();

    for i in 0..field_count {
        let key = format!("txn_f{}", i);
        // Mix ints and floats
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
        // initialize counters as ints
        profile = profile.with_field(key, Value::Int(0));
    }

    profile
}

fn generate_rules(rule_count: usize, txn_fields: usize, profile_fields: usize) -> String {
    let mut dsl = String::new();

    const CONDITIONS_PER_RULE: usize = 20;

    for i in 0..rule_count {
        let mut conds = Vec::with_capacity(CONDITIONS_PER_RULE);

        for c in 0..CONDITIONS_PER_RULE {
            let use_txn = c % 2 == 0;
            let idx = if use_txn { (i + c) % txn_fields } else { (i + c) % profile_fields };

            let op = match c % 6 {
                0 => ">",
                1 => "<",
                2 => ">=",
                3 => "<=",
                4 => "!=",
                _ => "==",
            };

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

fn bench_500_rules_large_payload(c: &mut Criterion) {
    // Parameters
    let txn_fields = 200usize;
    let profile_fields = 300usize;
    let rules = 500usize;

    // Generate DSL
    let dsl = generate_rules(rules, txn_fields, profile_fields);

    // Measure compilation time once (warm-up)
    let compile_start = Instant::now();
    let engine = RuleEngine::from_dsl(&dsl).expect("Compilation failed");
    let compile_duration = compile_start.elapsed();

    println!("Compiled {rules} rules (txn_fields={}, profile_fields={}) in {:?}", txn_fields, profile_fields, compile_duration);

    // Prepare data (set txn fields to large values so many rules trigger)
    let mut txn = generate_transaction(txn_fields);
    for i in 0..txn_fields {
        let key = format!("txn_f{}", i);
        // set to a large float so condition (>) typically true
        txn = txn.with_field(key, Value::Float(10_000.0 + (i as f64)));
    }

    let profile = generate_profile(profile_fields);

    // Warm-up executions
    for _ in 0..5 {
        let _ = engine.execute(txn.clone(), profile.clone());
    }

    // Benchmark execution-only
    c.bench_function("500_rules_200txn_300profile", |b| {
        b.iter(|| {
            engine.execute(black_box(txn.clone()), black_box(profile.clone()))
        })
    });

    // Also benchmark compilation cost separately
    c.bench_function("compile_500_rules_200_300", |b| {
        b.iter(|| {
            RuleEngine::from_dsl(black_box(&dsl)).unwrap()
        })
    });
}

criterion_group!(benches, bench_500_rules_large_payload);
criterion_main!(benches);
