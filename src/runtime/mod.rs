// src/runtime/mod.rs
//! Runtime components for executing bytecode

pub mod context;
pub mod value;
pub mod vm;

pub use context::ExecutionContext;
pub use value::Value;
pub use vm::VM;
