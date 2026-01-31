// src/compiler/mod.rs
//! Compiler that converts AST to bytecode

pub mod bytecode;
pub mod compiler;

use crate::parser::Program;
use crate::{CompiledFunction, CompiledRule, CompilationError};
use ahash::HashMap;

/// Compile a parsed program into bytecode
pub fn compile(program: Program) -> Result<(Vec<CompiledRule>, HashMap<String, CompiledFunction>), CompilationError> {
    let mut rules = Vec::new();
    let mut functions = HashMap::default();
    
    // Compile global functions
    for func in program.functions {
        let compiled = compiler::Compiler::compile_function(&func)?;
        functions.insert(compiled.name.clone(), compiled);
    }
    
    // Compile rules (sorted by priority, descending)
    let mut rule_nodes = program.rules;
    rule_nodes.sort_by(|a, b| b.priority.cmp(&a.priority));
    
    for rule in rule_nodes {
        let compiled = compiler::Compiler::compile_rule(&rule)?;
        rules.push(compiled);
    }
    
    Ok((rules, functions))
}
