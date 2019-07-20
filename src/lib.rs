#[macro_use] extern crate lazy_static;

pub mod error;
pub mod parser;
pub mod optimizer;
pub mod interpreter;

// Re-export common types
pub use error::BrainfuckError;
pub use parser::Instruction;
pub use optimizer::Optimizer;
pub use interpreter::Interpreter;