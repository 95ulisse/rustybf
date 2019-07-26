//! An optimizing compiler, interpreter and JIT for Brainfuck.
//! 
//! ## Example
//! 
//! ```rust
//! use rustybf::{Compiler, Interpreter, Optimizer};
//! use rustybf::parser::parse;
//! 
//! // Parse the source file
//! let file = File::open("hello_world.b").unwrap();
//! let mut instructions = parse(file).unwrap();
//! 
//! // Optimize the instructions
//! // (use `rustybf::optimizer::DEFAULT_OPTIMIZATION_PASSES` for the default passes)
//! let optimizer = Optimizer::with_passes_str("collapse-increments,mul-loops,dead-code").unwrap();
//! instructions = optimizer.run(instructions);
//! 
//! // Now we can ether prepare an interpreter to run the instructions, or...
//! let mut interpreter =
//!     Interpreter::builder()
//!     .input(std::io::stdin())
//!     .output(std::io::stdout())
//!     .build();
//! interpreter.run(&instructions).unwrap();
//! 
//! // ... JIT compile the program and jump right to it
//! let program =
//!     Compiler::new(3) // 3 is the LLVM optimization level
//!     .compile_instructions(&instructions)
//!     .finish();
//! program.run();
//! ```

#[macro_use] extern crate lazy_static;

pub mod error;
pub mod parser;
pub mod optimizer;
pub mod interpreter;
pub mod compiler;

// Re-export common types
pub use error::BrainfuckError;
pub use parser::Instruction;
pub use optimizer::Optimizer;
pub use interpreter::Interpreter;
pub use compiler::Compiler;