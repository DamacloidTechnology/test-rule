// benches/rule_execution.rs
//! Performance benchmarks for the rule engine
//! 
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value};

fn benchmark_single_rule(c: &mut Criterion) {
    let dsl = r#"
        rule "simple" {
            priority: 100,
            if (txn.amount > 1000) {
                setFraudScore(0.8);
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    let transaction = Transaction::new().with_field("amount", Value::Float(5000.0));
    let profile = UserProfile::new();
    
    c.bench_function("single_rule", |b| {
        b.iter(|| {
            engine.execute(
                black_box(transaction.clone()),
                black_box(profile.clone()),
            )
        })
    });
}

fn benchmark_10_rules(c: &mut Criterion) {
    let mut dsl = String::new();
    for i in 0..10 {
        dsl.push_str(&format!(
            r#"
            rule "rule_{}" {{
                priority: {},
                if (txn.amount > {}) {{
                    profile.counter = profile.counter + 1;
                }}
            }}
            "#,
            i, 100 - i, i * 100
        ));
    }
    
    let engine = RuleEngine::from_dsl(&dsl).unwrap();
    let transaction = Transaction::new().with_field("amount", Value::Float(5000.0));
    let profile = UserProfile::new().with_field("counter", Value::Int(0));
    
    c.bench_function("10_rules", |b| {
        b.iter(|| {
            engine.execute(
                black_box(transaction.clone()),
                black_box(profile.clone()),
            )
        })
    });
}

fn benchmark_100_rules(c: &mut Criterion) {
    let mut dsl = String::new();
    for i in 0..100 {
        dsl.push_str(&format!(
            r#"
            rule "rule_{}" {{
                priority: {},
                if (txn.amount > {}) {{
                    profile.counter = profile.counter + 1;
                }}
            }}
            "#,
            i, 1000 - i, i * 50
        ));
    }
    
    let engine = RuleEngine::from_dsl(&dsl).unwrap();
    let transaction = Transaction::new().with_field("amount", Value::Float(5000.0));
    let profile = UserProfile::new().with_field("counter", Value::Int(0));
    
    c.bench_function("100_rules", |b| {
        b.iter(|| {
            engine.execute(
                black_box(transaction.clone()),
                black_box(profile.clone()),
            )
        })
    });
}

fn benchmark_500_rules(c: &mut Criterion) {
    let mut dsl = String::new();
    for i in 0..500 {
        dsl.push_str(&format!(
            r#"
            rule "rule_{}" {{
                priority: {},
                if (txn.amount > {}) {{
                    profile.counter = profile.counter + 1;
                }}
            }}
            "#,
            i, 5000 - i, i * 10
        ));
    }
    
    let engine = RuleEngine::from_dsl(&dsl).unwrap();
    let transaction = Transaction::new().with_field("amount", Value::Float(5000.0));
    let profile = UserProfile::new().with_field("counter", Value::Int(0));
    
    c.bench_function("500_rules", |b| {
        b.iter(|| {
            engine.execute(
                black_box(transaction.clone()),
                black_box(profile.clone()),
            )
        })
    });
}

fn benchmark_complex_rules(c: &mut Criterion) {
    let dsl = r#"
        function calculateRiskScore(profile, txn) {
            let base_score = 0.0;
            
            if (profile.txn_count_1h > 10) {
                base_score = base_score + 0.3;
            }
            
            if (txn.amount > profile.avg_amount * 5) {
                base_score = base_score + 0.4;
            }
            
            if (txn.country != profile.home_country) {
                base_score = base_score + 0.2;
            }
            
            profile.risk_score = base_score;
        }
        
        rule "velocity_check" {
            priority: 100,
            if (profile.txn_count_1h > 50) {
                createCase("HIGH", "Excessive velocity");
                setFraudScore(0.9);
                return;
            }
        }
        
        rule "amount_spike" {
            priority: 90,
            if (txn.amount > profile.avg_amount * 10) {
                createCase("HIGH", "Amount spike");
                profile.spike_count = profile.spike_count + 1;
            }
        }
        
        rule "calculate_risk" {
            priority: 80,
            if (true) {
                calculateRiskScore(profile, txn);
                
                if (profile.risk_score > 0.7) {
                    setFraudScore(profile.risk_score);
                    createCase("MEDIUM", "High risk score");
                }
            }
        }
        
        rule "country_mismatch" {
            priority: 70,
            if (txn.country != profile.home_country) {
                if (profile.travel_mode == false) {
                    createCase("MEDIUM", "Country mismatch");
                }
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    let transaction = Transaction::new()
        .with_field("amount", Value::Float(5000.0))
        .with_field("country", Value::String("UK".to_string()));
    
    let profile = UserProfile::new()
        .with_field("txn_count_1h", Value::Int(15))
        .with_field("avg_amount", Value::Float(1000.0))
        .with_field("home_country", Value::String("US".to_string()))
        .with_field("travel_mode", Value::Bool(false))
        .with_field("spike_count", Value::Int(0))
        .with_field("risk_score", Value::Float(0.0));
    
    c.bench_function("complex_rules", |b| {
        b.iter(|| {
            engine.execute(
                black_box(transaction.clone()),
                black_box(profile.clone()),
            )
        })
    });
}

fn benchmark_profile_mutations(c: &mut Criterion) {
    let dsl = r#"
        rule "update_many_fields" {
            priority: 100,
            if (true) {
                profile.field1 = profile.field1 + 1;
                profile.field2 = profile.field2 + txn.amount;
                profile.field3 = profile.field3 + 1;
                profile.field4 = profile.field4 + txn.amount;
                profile.field5 = profile.field5 + 1;
                profile.field6 = profile.field6 + txn.amount;
                profile.field7 = profile.field7 + 1;
                profile.field8 = profile.field8 + txn.amount;
                profile.field9 = profile.field9 + 1;
                profile.field10 = profile.field10 + txn.amount;
            }
        }
    "#;
    
    let engine = RuleEngine::from_dsl(dsl).unwrap();
    
    let transaction = Transaction::new().with_field("amount", Value::Float(500.0));
    
    let mut profile = UserProfile::new();
    for i in 1..=10 {
        profile = profile.with_field(format!("field{}", i), Value::Int(0));
    }
    
    c.bench_function("profile_mutations", |b| {
        b.iter(|| {
            engine.execute(
                black_box(transaction.clone()),
                black_box(profile.clone()),
            )
        })
    });
}

fn benchmark_compilation(c: &mut Criterion) {
    let dsl = r#"
        rule "test" {
            priority: 100,
            if (txn.amount > 1000 && profile.risk_score > 0.5) {
                createCase("HIGH", "High risk transaction");
                setFraudScore(0.9);
                profile.alert_count = profile.alert_count + 1;
            }
        }
    "#;
    
    c.bench_function("compile_single_rule", |b| {
        b.iter(|| {
            RuleEngine::from_dsl(black_box(dsl)).unwrap()
        })
    });
}

fn benchmark_by_rule_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("rule_count_scaling");
    
    for rule_count in [1, 10, 50, 100, 250, 500].iter() {
        let mut dsl = String::new();
        for i in 0..*rule_count {
            dsl.push_str(&format!(
                r#"
                rule "rule_{}" {{
                    priority: {},
                    if (txn.amount > {}) {{
                        profile.counter = profile.counter + 1;
                    }}
                }}
                "#,
                i, 10000 - i, i * 10
            ));
        }
        
        let engine = RuleEngine::from_dsl(&dsl).unwrap();
        let transaction = Transaction::new().with_field("amount", Value::Float(5000.0));
        let profile = UserProfile::new().with_field("counter", Value::Int(0));
        
        group.bench_with_input(
            BenchmarkId::from_parameter(rule_count),
            rule_count,
            |b, _| {
                b.iter(|| {
                    engine.execute(
                        black_box(transaction.clone()),
                        black_box(profile.clone()),
                    )
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_rule,
    benchmark_10_rules,
    benchmark_100_rules,
    benchmark_500_rules,
    benchmark_complex_rules,
    benchmark_profile_mutations,
    benchmark_compilation,
    benchmark_by_rule_count,
);

criterion_main!(benches);
