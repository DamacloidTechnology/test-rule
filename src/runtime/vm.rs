// src/runtime/vm.rs
//! Virtual Machine that executes bytecode
//!
//! This is the HOT PATH - every nanosecond counts here!

use crate::compiler::bytecode::{ActionType, Instruction};
use crate::runtime::context::ExecutionContext;
use crate::{Action, CompiledFunction, Value};
use ahash::HashMap;

pub struct VM;

impl VM {
    /// Execute bytecode in the given context
    ///
    /// This is the performance-critical path!
    pub fn execute(
        bytecode: &[Instruction],
        ctx: &mut ExecutionContext,
        functions: &HashMap<String, CompiledFunction>,
    ) {
        let mut pc = 0; // Program counter

        while pc < bytecode.len() {
            let instruction = &bytecode[pc];

            match instruction {
                Instruction::Push(value) => {
                    ctx.push(value.clone());
                }

                Instruction::Pop => {
                    ctx.pop();
                }

                Instruction::Dup => {
                    if let Some(value) = ctx.peek() {
                        ctx.push(value.clone());
                    }
                }

                Instruction::LoadProfileField(field) => {
                    let value = ctx.get_profile_field(field);
                    ctx.push(value);
                }

                Instruction::StoreProfileField(field) => {
                    if let Some(value) = ctx.pop() {
                        ctx.set_profile_field(field.clone(), value);
                    }
                }

                Instruction::LoadTxnField(field) => {
                    let value = ctx.get_txn_field(field);
                    ctx.push(value);
                }

                Instruction::StoreTxnField(field) => {
                    if let Some(value) = ctx.pop() {
                        ctx.set_txn_field(field.clone(), value);
                    }
                }

                Instruction::LoadLocal(name) => {
                    let value = ctx.get_local(name);
                    ctx.push(value);
                }

                Instruction::StoreLocal(name) => {
                    if let Some(value) = ctx.pop() {
                        ctx.set_local(name.clone(), value);
                    }
                }

                Instruction::Add => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Self::add(a, b));
                    }
                }

                Instruction::Sub => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Self::sub(a, b));
                    }
                }

                Instruction::Mul => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Self::mul(a, b));
                    }
                }

                Instruction::Div => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Self::div(a, b));
                    }
                }

                Instruction::Mod => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Self::modulo(a, b));
                    }
                }

                Instruction::Neg => {
                    if let Some(a) = ctx.pop() {
                        ctx.push(Self::neg(a));
                    }
                }

                Instruction::Eq => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Value::Bool(Self::eq(&a, &b)));
                    }
                }

                Instruction::Ne => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Value::Bool(!Self::eq(&a, &b)));
                    }
                }

                Instruction::Gt => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Value::Bool(Self::gt(&a, &b)));
                    }
                }

                Instruction::Gte => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Value::Bool(Self::gt(&a, &b) || Self::eq(&a, &b)));
                    }
                }

                Instruction::Lt => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Value::Bool(Self::lt(&a, &b)));
                    }
                }

                Instruction::Lte => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Value::Bool(Self::lt(&a, &b) || Self::eq(&a, &b)));
                    }
                }

                Instruction::And => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Value::Bool(a.as_bool() && b.as_bool()));
                    }
                }

                Instruction::Or => {
                    if let (Some(b), Some(a)) = (ctx.pop(), ctx.pop()) {
                        ctx.push(Value::Bool(a.as_bool() || b.as_bool()));
                    }
                }

                Instruction::Not => {
                    if let Some(a) = ctx.pop() {
                        ctx.push(Value::Bool(!a.as_bool()));
                    }
                }

                Instruction::Jump(target) => {
                    pc = *target;
                    continue;
                }

                Instruction::JumpIfFalse(target) => {
                    if let Some(condition) = ctx.pop() {
                        if !condition.as_bool() {
                            pc = *target;
                            continue;
                        }
                    }
                }

                Instruction::Return => {
                    ctx.should_return = true;
                    break;
                }

                Instruction::CallGlobal(func_name, arg_count) => {
                    if let Some(func) = functions.get(func_name) {
                        // Pop arguments and store as locals
                        let mut args = Vec::new();
                        for _ in 0..*arg_count {
                            if let Some(arg) = ctx.pop() {
                                args.push(arg);
                            }
                        }
                        args.reverse(); // Arguments are in reverse order on stack

                        // Set up parameter bindings
                        for (i, param) in func.params.iter().enumerate() {
                            if let Some(arg) = args.get(i) {
                                ctx.set_local(param.clone(), arg.clone());
                            }
                        }

                        // Execute function bytecode
                        Self::execute(&func.bytecode, ctx, functions);
                    }
                }

                Instruction::CallAction(action_type, arg_count) => {
                    // Pop arguments
                    let mut args = Vec::new();
                    for _ in 0..*arg_count {
                        if let Some(arg) = ctx.pop() {
                            args.push(arg);
                        }
                    }
                    args.reverse();

                    // Create action based on type
                    let action = Self::create_action(action_type, args);
                    ctx.add_action(action);
                }

                Instruction::ArrayAccess => {
                    if let (Some(index), Some(array)) = (ctx.pop(), ctx.pop()) {
                        if let Value::Array(arr) = array {
                            let idx = index.as_int() as usize;
                            let value = arr.get(idx).cloned().unwrap_or(Value::Null);
                            ctx.push(value);
                        } else {
                            ctx.push(Value::Null);
                        }
                    }
                }

                Instruction::ArrayContains => {
                    if let (Some(item), Some(array)) = (ctx.pop(), ctx.pop()) {
                        if let Value::Array(arr) = array {
                            ctx.push(Value::Bool(arr.contains(&item)));
                        } else {
                            ctx.push(Value::Bool(false));
                        }
                    }
                }

                Instruction::ObjectGet(field) => {
                    if let Some(obj) = ctx.pop() {
                        if let Value::Object(map) = obj {
                            let value = map.get(field).cloned().unwrap_or(Value::Null);
                            ctx.push(value);
                        } else {
                            ctx.push(Value::Null);
                        }
                    }
                }

                Instruction::MethodCall(method, arg_count) => {
                    // Pop arguments
                    let mut args = Vec::new();
                    for _ in 0..*arg_count {
                        if let Some(arg) = ctx.pop() {
                            args.push(arg);
                        }
                    }
                    args.reverse();

                    // Pop object
                    if let Some(obj) = ctx.pop() {
                        let result = Self::call_method(&obj, method, args);
                        ctx.push(result);
                    }
                }
            }

            pc += 1;
        }
    }

    // Arithmetic operations
    #[inline]
    fn add(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x.wrapping_add(y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(x + y),
            (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 + y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x + y as f64),
            (Value::String(mut x), Value::String(y)) => {
                x.push_str(&y);
                Value::String(x)
            }
            _ => Value::Null,
        }
    }

    #[inline]
    fn sub(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x.wrapping_sub(y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(x - y),
            (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 - y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x - y as f64),
            _ => Value::Null,
        }
    }

    #[inline]
    fn mul(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x.wrapping_mul(y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(x * y),
            (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 * y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x * y as f64),
            _ => Value::Null,
        }
    }

    #[inline]
    fn div(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) if y != 0 => Value::Int(x / y),
            (Value::Float(x), Value::Float(y)) if y != 0.0 => Value::Float(x / y),
            (Value::Int(x), Value::Float(y)) if y != 0.0 => Value::Float(x as f64 / y),
            (Value::Float(x), Value::Int(y)) if y != 0 => Value::Float(x / y as f64),
            _ => Value::Null,
        }
    }

    #[inline]
    fn modulo(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) if y != 0 => Value::Int(x % y),
            _ => Value::Null,
        }
    }

    #[inline]
    fn neg(a: Value) -> Value {
        match a {
            Value::Int(x) => Value::Int(-x),
            Value::Float(x) => Value::Float(-x),
            _ => Value::Null,
        }
    }

    // Comparison operations
    #[inline]
    fn eq(a: &Value, b: &Value) -> bool {
        a == b
    }

    #[inline]
    fn gt(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x > y,
            (Value::Float(x), Value::Float(y)) => x > y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) > *y,
            (Value::Float(x), Value::Int(y)) => *x > (*y as f64),
            (Value::String(x), Value::String(y)) => x > y,
            _ => false,
        }
    }

    #[inline]
    fn lt(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x < y,
            (Value::Float(x), Value::Float(y)) => x < y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) < *y,
            (Value::Float(x), Value::Int(y)) => *x < (*y as f64),
            (Value::String(x), Value::String(y)) => x < y,
            _ => false,
        }
    }

    // Action creation
    fn create_action(action_type: &ActionType, args: Vec<Value>) -> Action {
        match action_type {
            ActionType::CreateCase => {
                let severity = args.get(0).map(|v| v.as_string()).unwrap_or_default();
                let reason = args.get(1).map(|v| v.as_string()).unwrap_or_default();
                let metadata = args.get(2).map(|v| v.as_object()).unwrap_or_default();

                Action::CreateCase {
                    severity,
                    reason,
                    metadata,
                }
            }
            ActionType::CreateComment => {
                let comment = args.get(0).map(|v| v.as_string()).unwrap_or_default();

                Action::CreateComment {
                    case_id: None,
                    comment,
                }
            }
            ActionType::SendAuthAdvise => {
                let channel = args.get(0).map(|v| v.as_string()).unwrap_or_default();
                let template = args.get(1).map(|v| v.as_string()).unwrap_or_default();
                let params = args.get(2).map(|v| v.as_object()).unwrap_or_default();

                Action::SendAuthAdvise {
                    channel,
                    template,
                    params,
                }
            }
            ActionType::SetFraudScore => {
                let score = args.get(0).map(|v| v.as_float()).unwrap_or(0.0);

                Action::SetFraudScore { score }
            }
            ActionType::SetDecision => {
                let decision = args.get(0).map(|v| v.as_string()).unwrap_or_default();

                Action::SetDecision { decision }
            }
            ActionType::Custom(name) => {
                let mut params = HashMap::default();
                for (i, arg) in args.iter().enumerate() {
                    params.insert(format!("arg{}", i), arg.clone());
                }

                Action::Custom {
                    action_name: name.clone(),
                    params,
                }
            }
        }
    }

    // Method calls
    fn call_method(obj: &Value, method: &str, args: Vec<Value>) -> Value {
        match (obj, method) {
            (Value::Array(arr), "length") => Value::Int(arr.len() as i64),
            (Value::String(s), "length") => Value::Int(s.len() as i64),
            _ => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Transaction, UserProfile};

    #[test]
    fn test_arithmetic() {
        let mut ctx = ExecutionContext::new(Transaction::new(), UserProfile::new());

        let bytecode = vec![
            Instruction::Push(Value::Int(10)),
            Instruction::Push(Value::Int(5)),
            Instruction::Add,
        ];

        VM::execute(&bytecode, &mut ctx, &HashMap::default());

        assert_eq!(ctx.pop(), Some(Value::Int(15)));
    }

    #[test]
    fn test_comparison() {
        let mut ctx = ExecutionContext::new(Transaction::new(), UserProfile::new());

        let bytecode = vec![
            Instruction::Push(Value::Int(10)),
            Instruction::Push(Value::Int(5)),
            Instruction::Gt,
        ];

        VM::execute(&bytecode, &mut ctx, &HashMap::default());

        assert_eq!(ctx.pop(), Some(Value::Bool(true)));
    }

    #[test]
    fn test_profile_access() {
        let mut ctx = ExecutionContext::new(
            Transaction::new(),
            UserProfile::new().with_field("count", Value::Int(5)),
        );

        let bytecode = vec![
            Instruction::LoadProfileField("count".to_string()),
            Instruction::Push(Value::Int(1)),
            Instruction::Add,
            Instruction::StoreProfileField("count".to_string()),
        ];

        VM::execute(&bytecode, &mut ctx, &HashMap::default());

        assert_eq!(ctx.get_profile_field("count"), Value::Int(6));
    }
}
