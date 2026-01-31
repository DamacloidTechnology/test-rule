// src/compiler/bytecode.rs
//! Bytecode instructions for the rule engine VM

use crate::Value;
use serde::{Deserialize, Serialize};

/// Bytecode instructions executed by the VM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    // Stack operations
    Push(Value),
    Pop,
    Dup,
    
    // Variable access
    LoadProfileField(String),
    StoreProfileField(String),
    LoadTxnField(String),
    StoreTxnField(String),
    LoadLocal(String),
    StoreLocal(String),
    
    // Arithmetic operations
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    
    // Comparison operations
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    
    // Logical operations
    And,
    Or,
    Not,
    
    // Control flow
    Jump(usize),
    JumpIfFalse(usize),
    Return,
    
    // Function and action calls
    CallGlobal(String, usize), // function name, arg count
    CallAction(ActionType, usize), // action type, arg count
    
    // Array/Object operations
    ArrayAccess,
    ArrayContains,
    ObjectGet(String),
    
    // Method calls
    MethodCall(String, usize), // method name, arg count
}

/// Action types that can be called from rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionType {
    CreateCase,
    CreateComment,
    SendAuthAdvise,
    SetFraudScore,
    SetDecision,
    Custom(String),
}

impl Instruction {
    /// Returns true if this instruction is a jump target
    pub fn is_jump(&self) -> bool {
        matches!(self, Instruction::Jump(_) | Instruction::JumpIfFalse(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_serialization() {
        let inst = Instruction::Push(Value::Int(42));
        let bytes = bincode::serialize(&inst).unwrap();
        let decoded: Instruction = bincode::deserialize(&bytes).unwrap();
        assert_eq!(inst, decoded);
    }
}
