// src/parser/mod.rs
//! Parser for the fraud rule DSL
//! 
//! Converts DSL source code into an Abstract Syntax Tree (AST)

pub mod ast;
pub mod lexer;
pub mod parser;

use crate::CompilationError;
pub use ast::Program;

/// Parse DSL source code into an AST
pub fn parse(source: &str) -> Result<Program, CompilationError> {
    let mut parser = parser::Parser::new(source)
        .map_err(|e| CompilationError::ParseError(e.to_string()))?;
    
    parser.parse()
        .map_err(|e| CompilationError::ParseError(e.to_string()))
}
