pub mod parser;
pub mod interpreter;

#[derive(Debug)]
pub enum BrainfuckError {
    /// I/O error.
    IoError(std::io::Error),
    /// Mismatched brackets while parsing.
    MismatchedBrackets,
    /// The data pointer underflowed the available tape.
    TapeUnderflow,
    /// The data pointer overflowed the available tape.
    TapeOverflow
}