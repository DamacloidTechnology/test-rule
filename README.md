# Fraud Rule Engine

A high-performance, stateless rule engine for fraud detection in banking systems. Written in Rust for maximum performance and safety.

## 🎯 Key Features

- **Ultra-Fast**: Process 500+ rules in <2ms (P99 latency)
- **High Throughput**: 10,000+ TPS on a single node
- **Stateless Design**: Pure function execution with no side effects
- **Hot Reload**: Deploy rules without downtime using bytecode serialization
- **Type-Safe**: Strongly typed with compile-time guarantees
- **Simple DSL**: JavaScript-like syntax for easy rule authoring
- **Horizontally Scalable**: Stateless design enables linear scaling

## 📊 Performance Benchmarks

```
single_rule         time: [1.2 µs]
10_rules            time: [8.5 µs]
100_rules           time: [82 µs]
500_rules           time: [410 µs]
complex_rules       time: [15 µs]
```

*Benchmarks run on AMD Ryzen 9 5900X, single-threaded*

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
fraud-rule-engine = "0.1.0"
```

### Basic Usage

```rust
use fraud_rule_engine::{RuleEngine, Transaction, UserProfile, Value};

// Define rules in DSL
let dsl = r#"
    rule "high_amount" {
        priority: 100,
        if (txn.amount > 1000) {
            createCase("HIGH", "Large transaction");
            setFraudScore(0.8);
        }
    }
"#;

// Compile rules once at startup
let engine = RuleEngine::from_dsl(dsl)?;

// Execute for each transaction (fast!)
let transaction = Transaction::new()
    .with_field("amount", Value::Float(5000.0));

let profile = UserProfile::new()
    .with_field("avg_amount", Value::Float(500.0));

let result = engine.execute(transaction, profile);

// Process results
for action in result.actions {
    // Execute actions asynchronously
    match action {
        Action::CreateCase { severity, reason, .. } => {
            create_fraud_case(severity, reason).await;
        }
        Action::SetFraudScore { score } => {
            update_fraud_score(score).await;
        }
        _ => {}
    }
}

// Update profile in database
update_user_profile(result.profile).await;
```

## 📝 Rule DSL Syntax

### Why Just `if/else` Instead of `when/then`?

We use simple `if/else` statements instead of `when/then` blocks because:

1. **Familiarity**: `if/else` is universal across programming languages
2. **Simplicity**: Less cognitive overhead for rule authors
3. **Flexibility**: Easier to nest conditions and handle complex logic
4. **No Ambiguity**: Clear control flow that matches developer expectations

### Rule Structure

```javascript
rule "rule_name" {
    priority: 100,           // Higher priority = executes first
    enabled: true,           // Can be disabled without recompilation
    
    // Use simple if/else for conditions
    if (condition) {
        // Actions to take
        profile.field = value;
        createCase("HIGH", "Reason");
        setFraudScore(0.8);
    } else {
        // Alternative actions
        setFraudScore(0.1);
    }
}
```

### Global Functions

```javascript
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
```

### Available Actions

- `createCase(severity, reason)` - Create a fraud case
- `createComment(comment)` - Add investigation comment
- `sendAuthAdvise(channel, template)` - Send customer notification
- `setFraudScore(score)` - Set fraud risk score (0.0 - 1.0)
- `setDecision(decision)` - Set decision ("ALLOW", "BLOCK", "REVIEW")
- `return` - Short-circuit execution (stop processing rules)

### Data Access

- **Transaction fields**: `txn.amount`, `txn.country`, `txn.merchant`, etc.
- **Profile fields**: `profile.txn_count_1h`, `profile.risk_score`, etc.
- **Profile mutation**: `profile.field = value`
- **Transaction mutation**: `txn.field = value`

### Examples

#### Velocity Check

```javascript
rule "high_velocity" {
    priority: 100,
    if (profile.txn_count_1h > 50) {
        createCase("HIGH", "Excessive velocity");
        setFraudScore(0.9);
        return; // Don't process other rules
    }
}
```

#### Amount Spike

```javascript
rule "amount_spike" {
    priority: 90,
    if (txn.amount > profile.avg_amount * 10) {
        createCase("HIGH", "Amount spike detected");
        profile.spike_count = profile.spike_count + 1;
        setFraudScore(0.8);
    }
}
```

#### Country Mismatch

```javascript
rule "country_check" {
    priority: 80,
    if (txn.country != profile.home_country) {
        if (profile.travel_mode == false) {
            createCase("MEDIUM", "Unexpected country");
            sendAuthAdvise("SMS", "location_verification");
        }
    }
}
```

#### Complex Risk Calculation

```javascript
function calculateRisk(profile, txn) {
    let risk = 0.0;
    
    if (profile.txn_count_1h > 10) {
        risk = risk + 0.3;
    }
    
    if (txn.amount > profile.avg_amount * 5) {
        risk = risk + 0.4;
    }
    
    if (txn.country != profile.home_country) {
        risk = risk + 0.2;
    }
    
    profile.risk_score = risk;
}

rule "risk_assessment" {
    priority: 100,
    if (true) {
        calculateRisk(profile, txn);
        
        if (profile.risk_score > 0.7) {
            createCase("HIGH", "High risk score");
            setFraudScore(profile.risk_score);
        }
    }
}
```

## 🏗️ Architecture

### Stateless Library Design

The rule engine is designed as a **pure, stateless library**:

```
Input:  Transaction + UserProfile
Output: ExecutionResult (modified profile, actions to execute)
```

This design enables:
- **No I/O operations** in the rule engine itself
- **Easy testing** with deterministic results
- **Horizontal scaling** with zero coordination
- **Integration flexibility** with any async runtime

### Recommended Integration Pattern

```rust
// Your async service
async fn process_transaction(txn_id: &str) {
    // 1. Fetch data (async)
    let transaction = fetch_transaction(txn_id).await;
    let profile = fetch_user_profile(&transaction.user_id).await;
    
    // 2. Execute rules (sync, fast!)
    let result = rule_engine.execute(transaction, profile);
    
    // 3. Execute actions (async)
    for action in result.actions {
        execute_action(action).await;
    }
    
    // 4. Update profile (async)
    update_profile(result.profile).await;
}
```

### Bytecode Compilation

Rules are compiled to bytecode for maximum performance:

```
DSL Source → Lexer → Parser → AST → Compiler → Bytecode → VM Execution
```

The bytecode can be serialized for hot reload:

```rust
// Compile and save
let engine = RuleEngine::from_dsl(dsl)?;
let bytecode = engine.to_bytecode()?;
redis.set("rules:v1", bytecode).await?;

// Load and execute (zero downtime!)
let bytecode = redis.get("rules:v1").await?;
let engine = RuleEngine::from_bytecode(&bytecode)?;
```

## 🔥 Hot Reload (Phase 2)

Deploy new rules without downtime:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

struct FraudService {
    engine: Arc<RwLock<RuleEngine>>,
}

impl FraudService {
    async fn reload_rules(&self, new_dsl: &str) {
        let new_engine = RuleEngine::from_dsl(new_dsl).unwrap();
        
        // Atomic swap - no requests dropped!
        let mut engine = self.engine.write().await;
        *engine = new_engine;
    }
    
    async fn process(&self, txn: Transaction, profile: UserProfile) {
        let engine = self.engine.read().await;
        let result = engine.execute(txn, profile);
        // ... process results
    }
}
```

## 🧪 Testing

### Run Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# All tests with output
cargo test -- --nocapture
```

### Run Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench -- single_rule

# Generate HTML report
cargo bench --features html_reports
```

## 📈 Performance Tips

1. **Pre-compile rules at startup**: Compilation is relatively expensive (~100µs per rule)
2. **Reuse the same engine**: Clone is cheap (Arc-wrapped)
3. **Use bytecode for hot reload**: Deserialization is faster than compilation
4. **Batch profile updates**: Write profile changes asynchronously
5. **Profile before optimizing**: Use the built-in execution metadata

## 🛠️ Development

### Project Structure

```
fraud-rule-engine/
├── src/
│   ├── lib.rs              # Public API
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── lexer.rs        # Tokenization
│   │   ├── parser.rs       # Parse tokens → AST
│   │   └── ast.rs          # AST definitions
│   ├── compiler/
│   │   ├── mod.rs
│   │   ├── bytecode.rs     # Instruction set
│   │   └── compiler.rs     # AST → Bytecode
│   ├── runtime/
│   │   ├── mod.rs
│   │   ├── vm.rs           # Virtual machine
│   │   ├── context.rs      # Execution context
│   │   └── value.rs        # Dynamic values
│   └── actions/
│       └── mod.rs          # Action definitions
├── tests/
│   └── integration_tests.rs
├── benches/
│   └── rule_execution.rs
└── examples/
    ├── basic_usage.rs
    └── async_integration.rs
```

### Build

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run example
cargo run --example basic_usage

# Run async example
cargo run --example async_integration
```

## 🎓 Examples

See the `examples/` directory for complete examples:

- `basic_usage.rs` - Simple rule execution
- `async_integration.rs` - Full async service integration with hot reload

Run examples:

```bash
cargo run --example basic_usage
cargo run --example async_integration
```

## 📋 Roadmap

### Phase 1: Core Engine ✅
- [x] Rule DSL with if/else syntax
- [x] Bytecode compiler
- [x] Virtual machine
- [x] Profile mutations
- [x] Action system
- [x] Global functions
- [x] Short-circuit execution

### Phase 2: Hot Reload ✅
- [x] Bytecode serialization
- [x] Rule versioning
- [x] Zero-downtime deployment
- [x] Async integration examples

### Phase 3: Advanced Features (Future)
- [ ] Rule simulation/testing UI
- [ ] A/B testing framework
- [ ] Rule dependency analysis
- [ ] Performance profiling UI
- [ ] Distributed tracing integration

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

MIT License - see LICENSE file for details

## 🙋 FAQ

### Q: Why Rust instead of Java/Python?
A: Sub-millisecond latency requirements demand zero-GC languages. Rust provides both performance and safety.

### Q: Can I use this with other languages?
A: Yes! You can wrap this as a C library using FFI, then call from any language.

### Q: How do I handle database connections?
A: The rule engine is stateless - handle all I/O in your service layer before/after rule execution.

### Q: What about concurrent rule execution?
A: The engine is thread-safe. Clone the Arc-wrapped engine and process transactions in parallel.

### Q: Can rules call external APIs?
A: No - rules should be pure logic. Emit custom actions and handle API calls in your service layer.

## 📞 Support

For questions and support, please open an issue on GitHub.
