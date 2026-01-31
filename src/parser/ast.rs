// src/parser/ast.rs
//! Abstract Syntax Tree definitions for the rule DSL

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<FunctionNode>,
    pub rules: Vec<RuleNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleNode {
    pub id: String,
    pub priority: i32,
    pub enabled: bool,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// if (condition) { ... } else { ... }
    IfStatement {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    
    /// variable assignment: profile.field = value
    Assignment {
        target: String,
        value: Expression,
    },
    
    /// Action call: createCase("HIGH", "reason")
    ActionCall {
        action: String,
        args: Vec<Expression>,
    },
    
    /// return; (short-circuit)
    Return,
    
    /// Expression statement (function call, etc.)
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Binary operation: a + b, a > b, etc.
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    
    /// Unary operation: !a, -a
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
    },
    
    /// Field access: profile.field, txn.amount
    FieldAccess {
        object: String,
        field: String,
    },
    
    /// Array access: array[index]
    ArrayAccess {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    
    /// Function call: calculateScore(profile, txn)
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
    
    /// Method call: array.includes(value)
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
    
    /// Literal value
    Literal(Literal),
    
    /// Variable reference
    Variable(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    
    // Comparison
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    
    // Logical
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl From<Literal> for crate::Value {
    fn from(lit: Literal) -> Self {
        match lit {
            Literal::Null => crate::Value::Null,
            Literal::Bool(b) => crate::Value::Bool(b),
            Literal::Int(n) => crate::Value::Int(n),
            Literal::Float(f) => crate::Value::Float(f),
            Literal::String(s) => crate::Value::String(s),
        }
    }
}
