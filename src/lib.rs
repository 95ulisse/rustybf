pub mod parser;

#[derive(Debug)]
pub enum BrainfuckError {
    IoError(std::io::Error),
    MismatchedBrackets
}