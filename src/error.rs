use std::error::Error;
use std::{fmt, io};
use crate::parser::Position;

#[derive(Debug)]
pub enum BrainfuckError {
    /// Generic message
    Message(String),
    /// I/O error.
    IoError(io::Error),
    /// Error while parsing.
    ParseError { message: String, position: Position },
    /// Unknown optimization pass.
    UnknownOptimizationPass(String),
    /// The data pointer underflowed the available tape.
    TapeUnderflow,
    /// The data pointer overflowed the available tape.
    TapeOverflow
}

impl Error for BrainfuckError {}

impl fmt::Display for BrainfuckError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BrainfuckError::*;
        match self {
            Message(ref m) => {
                write!(f, "{}", m)
            },
            IoError(ref e) => {
                write!(f, "I/O error: {}", e)
            },
            ParseError { ref message, position } => {
                write!(f, "Error parsing Brainfuck file: {} at ({}-{})", message, position.start, position.end)
            },
            UnknownOptimizationPass(ref name) => {
                write!(f, "Unknown optimization pass: {}", name)
            },
            TapeUnderflow => {
                write!(f, "Tape underflow")
            },
            TapeOverflow => {
                write!(f, "Tape overflow")
            }
        }
    }
}

impl From<&str> for BrainfuckError {
    fn from(s: &str) -> Self {
        BrainfuckError::Message(s.to_owned())
    }
}

impl From<String> for BrainfuckError {
    fn from(s: String) -> Self {
        BrainfuckError::Message(s)
    }
}

impl From<io::Error> for BrainfuckError {
    fn from(e: io::Error) -> Self {
        BrainfuckError::IoError(e)
    }
}