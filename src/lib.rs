pub mod parser;
pub mod interpreter;

use parser::Position;

#[derive(Debug)]
pub enum BrainfuckError {
    /// I/O error.
    IoError(std::io::Error),
    /// Error while parsing.
    ParseError { message: String, position: Position },
    /// The data pointer underflowed the available tape.
    TapeUnderflow,
    /// The data pointer overflowed the available tape.
    TapeOverflow
}