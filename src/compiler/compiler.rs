// src/compiler/compiler.rs
//! Compiler that converts AST to bytecode

use crate::compiler::bytecode::{ActionType, Instruction};
use crate::parser::ast::*;
use crate::{CompiledFunction, CompiledRule, CompilationError, Value};

pub struct Compiler {
    instructions: Vec<Instruction>,
    label_counter: usize,
    labels: Vec<(usize, usize)>, // (label_id, instruction_index)
}

impl Compiler {
    fn new() -> Self {
        Self {
            instructions: Vec::new(),
            label_counter: 0,
            labels: Vec::new(),
        }
    }
    
    pub fn compile_rule(rule: &RuleNode) -> Result<CompiledRule, CompilationError> {
        let mut compiler = Compiler::new();
        
        // Compile all statements in the rule body
        for stmt in &rule.body {
            compiler.compile_statement(stmt)?;
        }
        
        // Resolve jump labels
        let bytecode = compiler.resolve_labels();
        
        Ok(CompiledRule {
            id: rule.id.clone(),
            priority: rule.priority,
            enabled: rule.enabled,
            bytecode,
        })
    }
    
    pub fn compile_function(func: &FunctionNode) -> Result<CompiledFunction, CompilationError> {
        let mut compiler = Compiler::new();
        
        // Compile function body
        for stmt in &func.body {
            compiler.compile_statement(stmt)?;
        }
        
        let bytecode = compiler.resolve_labels();
        
        Ok(CompiledFunction {
            name: func.name.clone(),
            params: func.params.clone(),
            bytecode,
        })
    }
    
    fn compile_statement(&mut self, stmt: &Statement) -> Result<(), CompilationError> {
        match stmt {
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
            } => {
                // Compile condition
                self.compile_expression(condition)?;
                
                // Create labels
                let else_label = self.new_label();
                let end_label = self.new_label();
                
                // Jump to else if condition is false
                self.emit_jump_if_false(else_label);
                
                // Compile then block
                for stmt in then_block {
                    self.compile_statement(stmt)?;
                }
                
                // Jump to end
                self.emit_jump(end_label);
                
                // Else block
                self.place_label(else_label);
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.compile_statement(stmt)?;
                    }
                }
                
                // End label
                self.place_label(end_label);
            }
            
            Statement::Assignment { target, value } => {
                // Compile value expression
                self.compile_expression(value)?;
                
                // Determine storage location
                if target.starts_with("profile.") {
                    let field = target.strip_prefix("profile.").unwrap();
                    self.emit(Instruction::StoreProfileField(field.to_string()));
                } else if target.starts_with("txn.") || target.starts_with("transaction.") {
                    let field = target
                        .strip_prefix("txn.")
                        .or_else(|| target.strip_prefix("transaction."))
                        .unwrap();
                    self.emit(Instruction::StoreTxnField(field.to_string()));
                } else {
                    self.emit(Instruction::StoreLocal(target.clone()));
                }
            }
            
            Statement::ActionCall { action, args } => {
                // Compile arguments
                for arg in args {
                    self.compile_expression(arg)?;
                }
                
                // Emit action call
                let action_type = match action.as_str() {
                    "createCase" => ActionType::CreateCase,
                    "createComment" => ActionType::CreateComment,
                    "sendAuthAdvise" => ActionType::SendAuthAdvise,
                    "setFraudScore" => ActionType::SetFraudScore,
                    "setDecision" => ActionType::SetDecision,
                    _ => ActionType::Custom(action.clone()),
                };
                
                self.emit(Instruction::CallAction(action_type, args.len()));
            }
            
            Statement::Return => {
                self.emit(Instruction::Return);
            }
            
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                self.emit(Instruction::Pop); // Discard result
            }
        }
        
        Ok(())
    }
    
    fn compile_expression(&mut self, expr: &Expression) -> Result<(), CompilationError> {
        match expr {
            Expression::Binary { left, op, right } => {
                self.compile_expression(left)?;
                self.compile_expression(right)?;
                
                let instruction = match op {
                    BinaryOp::Add => Instruction::Add,
                    BinaryOp::Sub => Instruction::Sub,
                    BinaryOp::Mul => Instruction::Mul,
                    BinaryOp::Div => Instruction::Div,
                    BinaryOp::Mod => Instruction::Mod,
                    BinaryOp::Eq => Instruction::Eq,
                    BinaryOp::Ne => Instruction::Ne,
                    BinaryOp::Gt => Instruction::Gt,
                    BinaryOp::Gte => Instruction::Gte,
                    BinaryOp::Lt => Instruction::Lt,
                    BinaryOp::Lte => Instruction::Lte,
                    BinaryOp::And => Instruction::And,
                    BinaryOp::Or => Instruction::Or,
                };
                
                self.emit(instruction);
            }
            
            Expression::Unary { op, operand } => {
                self.compile_expression(operand)?;
                
                match op {
                    UnaryOp::Not => self.emit(Instruction::Not),
                    UnaryOp::Neg => self.emit(Instruction::Neg),
                }
            }
            
            Expression::FieldAccess { object, field } => {
                match object.as_str() {
                    "profile" => {
                        self.emit(Instruction::LoadProfileField(field.clone()));
                    }
                    "txn" | "transaction" => {
                        self.emit(Instruction::LoadTxnField(field.clone()));
                    }
                    _ => {
                        return Err(CompilationError::UnknownField(format!(
                            "{}.{}",
                            object, field
                        )));
                    }
                }
            }
            
            Expression::ArrayAccess { array, index } => {
                self.compile_expression(array)?;
                self.compile_expression(index)?;
                self.emit(Instruction::ArrayAccess);
            }
            
            Expression::FunctionCall { name, args } => {
                // Compile arguments
                for arg in args {
                    self.compile_expression(arg)?;
                }
                
                self.emit(Instruction::CallGlobal(name.clone(), args.len()));
            }
            
            Expression::MethodCall {
                object,
                method,
                args,
            } => {
                // Compile object
                self.compile_expression(object)?;
                
                // Compile arguments
                for arg in args {
                    self.compile_expression(arg)?;
                }
                
                // Special handling for common array methods
                if method == "includes" || method == "contains" {
                    self.emit(Instruction::ArrayContains);
                } else {
                    self.emit(Instruction::MethodCall(method.clone(), args.len()));
                }
            }
            
            Expression::Literal(lit) => {
                let value: Value = lit.clone().into();
                self.emit(Instruction::Push(value));
            }
            
            Expression::Variable(name) => {
                self.emit(Instruction::LoadLocal(name.clone()));
            }
        }
        
        Ok(())
    }
    
    fn emit(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }
    
    fn emit_jump(&mut self, label: usize) {
        self.emit(Instruction::Jump(label));
    }
    
    fn emit_jump_if_false(&mut self, label: usize) {
        self.emit(Instruction::JumpIfFalse(label));
    }
    
    fn new_label(&mut self) -> usize {
        let label = self.label_counter;
        self.label_counter += 1;
        label
    }
    
    fn place_label(&mut self, label: usize) {
        let position = self.instructions.len();
        self.labels.push((label, position));
    }
    
    fn resolve_labels(mut self) -> Vec<Instruction> {
        // Replace label IDs with actual instruction indices
        for instruction in &mut self.instructions {
            match instruction {
                Instruction::Jump(label) | Instruction::JumpIfFalse(label) => {
                    if let Some((_, pos)) = self.labels.iter().find(|(l, _)| l == label) {
                        *label = *pos;
                    }
                }
                _ => {}
            }
        }
        
        self.instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_assignment() {
        let rule = RuleNode {
            id: "test".to_string(),
            priority: 100,
            enabled: true,
            body: vec![Statement::Assignment {
                target: "profile.count".to_string(),
                value: Expression::Literal(Literal::Int(42)),
            }],
        };
        
        let compiled = Compiler::compile_rule(&rule).unwrap();
        assert!(!compiled.bytecode.is_empty());
    }

    #[test]
    fn test_compile_if_statement() {
        let rule = RuleNode {
            id: "test".to_string(),
            priority: 100,
            enabled: true,
            body: vec![Statement::IfStatement {
                condition: Expression::Literal(Literal::Bool(true)),
                then_block: vec![Statement::Return],
                else_block: None,
            }],
        };
        
        let compiled = Compiler::compile_rule(&rule).unwrap();
        
        // Should have: Push(true), JumpIfFalse, Return, label
        assert!(compiled.bytecode.len() >= 2);
    }
}
