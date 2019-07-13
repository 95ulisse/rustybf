use std::io::Read;
use std::{cmp, fmt, u8};
use crate::BrainfuckError;

/// Position range to track instructions back to source code.
/// Both ends are inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub start: usize,
    pub end: usize
}

impl From<usize> for Position {
    fn from(i: usize) -> Self {
        Position {
            start: i,
            end: i
        }
    }
}

impl Position {

    /// Merges two positions into one.
    pub fn merge(&self, other: Position) -> Position {
        let start = cmp::min(self.start, other.start);
        let end = cmp::max(self.end, other.end);
        Position { start, end }
    }

}

/// A single Brainfuck instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Add {
        amount: u8,
        position: Position
    },
    Right {
        amount: usize,
        position: Position
    },
    Left {
        amount: usize,
        position: Position
    },
    Input {
        position: Position
    },
    Output {
        position: Position
    },
    Loop {
        body: Vec<Instruction>,
        position: Position
    },

    // The following instructions are not part of the Brainfuck language,
    // but are added by the different optimizations

    Clear {
        position: Position
    },

    Mul {
        offset: isize,
        amount: u8,
        position: Position
    }
}

impl Instruction {
    
    /// Returns the position of this instruction in the source code.
    pub fn position(&self) -> Position {
        match *self {
             Instruction::Add { position, .. } => position,
             Instruction::Right { position, .. } => position,
             Instruction::Left { position, .. } => position,
             Instruction::Input { position, .. } => position,
             Instruction::Output { position, .. } => position,
             Instruction::Loop { position, .. } => position,
             Instruction::Clear { position, .. } => position,
             Instruction::Mul { position, .. } => position
        }
    }

    /// Returns `true` if the instruction represents a Brainfuck loop.
    /// Some instructions like `Clear` and `Mul` do not exist natively in the language,
    /// and are actually implemented with simple loops.
    pub fn is_loop(&self) -> bool {
        match *self {
            Instruction::Loop { .. } |
            Instruction::Clear { .. } |
            Instruction::Mul { .. }
                => true,

            _ => false
        }
    }

}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        print_instruction(self, f, 0)
    }
}

fn print_instruction(instruction: &Instruction, f: &mut fmt::Formatter, level: usize) -> fmt::Result {
    if level > 0 {
        write!(f, "{:width$}", "", width = level * 4)?;
    }
    match instruction {
        Instruction::Add { amount, .. } => {
            write!(f, "Add({})", amount)?;
        },
        Instruction::Right { amount, .. } => {
            write!(f, "Right({})", amount)?;
        },
        Instruction::Left { amount, .. } => {
            write!(f, "Left({})", amount)?;
        },
        Instruction::Input { .. } => {
            write!(f, "Input")?;
        },
        Instruction::Output { .. } => {
            write!(f, "Output")?;
        },
        Instruction::Loop { ref body, .. } => {
            writeln!(f, "Loop {{")?;
            for i in body {
                print_instruction(i, f, level + 1)?;
                writeln!(f)?;
            }
            write!(f, "{:width$}}}", "", width = level * 4)?;
        },
        Instruction::Clear { .. } => {
            write!(f, "Clear")?;
        },
        Instruction::Mul { offset, amount, .. } => {
            write!(f, "Mul({}) {{ offset: {} }}", amount, offset)?;
        }
    }
    Ok(())
}

/// Parses a Brainfuck program from the given stream.
pub fn parse(r: impl Read) -> Result<Vec<Instruction>, BrainfuckError> {

    let mut instructions: Vec<Instruction> = Vec::new();
    let mut stack: Vec<(Vec<Instruction>, usize)> = Vec::new();

    for (index, res) in r.bytes().enumerate() {
        match res {
            Err(e) => return Err(BrainfuckError::IoError(e)),
            Ok(b'>') => instructions.push(Instruction::Right  { position: index.into(), amount: 1 }),
            Ok(b'<') => instructions.push(Instruction::Left   { position: index.into(), amount: 1 }),
            Ok(b'+') => instructions.push(Instruction::Add    { position: index.into(), amount: 1  }),
            Ok(b'-') => instructions.push(Instruction::Add    { position: index.into(), amount: u8::MAX }),
            Ok(b'.') => instructions.push(Instruction::Output { position: index.into() }),
            Ok(b',') => instructions.push(Instruction::Input  { position: index.into() }),
            Ok(b'[') => {
                stack.push((instructions, index));
                instructions = Vec::new();
            },
            Ok(b']') => {
                if let Some((mut parent_instructions, parent_index)) = stack.pop() {
                    parent_instructions.push(Instruction::Loop {
                        body: instructions,
                        position: Position {
                            start: parent_index,
                            end: index
                        }
                    });
                    instructions = parent_instructions;
                } else {
                    return Err(BrainfuckError::ParseError {
                        message: "This ] has no matching opening [.".to_owned(),
                        position: index.into()
                    });
                }
            },
            Ok(_) => { /* Ignore every other character */ }
        }
    }

    if let Some((_, index)) = stack.pop() {
        return Err(BrainfuckError::ParseError {
            message: "This [ has no matching closing ].".to_owned(),
            position: index.into()
        });
    }

    Ok(instructions)
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_empty_program() {
        let prog = Cursor::new("");
        assert_eq!(parse(prog).unwrap(), vec![]);
    }

    #[test]
    fn test_simple_parse() {
        let prog = Cursor::new("+-><.,");
        assert_eq!(parse(prog).unwrap(), vec![
            Instruction::Add { amount: 1, position: 0.into() },
            Instruction::Add { amount: u8::MAX, position: 1.into() },
            Instruction::Right { position: 2.into(), amount: 1 },
            Instruction::Left { position: 3.into(), amount: 1 },
            Instruction::Output { position: 4.into() },
            Instruction::Input { position: 5.into() }
        ]);
    }

    #[test]
    fn test_empty_loop() {
        let prog = Cursor::new("[]");
        assert_eq!(parse(prog).unwrap(), vec![
            Instruction::Loop {
                body: vec![],
                position: Position { start: 0, end: 1 }
            }
        ]);
    }

    #[test]
    fn test_nested_loop() {
        let prog = Cursor::new("[+[,][+[.]-]-]");
        assert_eq!(parse(prog).unwrap(), vec![
            Instruction::Loop {
                position: Position { start: 0, end: 13 },
                body: vec![
                    Instruction::Add { amount: 1, position: 1.into() },
                    Instruction::Loop{
                        position: Position { start: 2, end: 4 },
                        body: vec![
                            Instruction::Input { position: 3.into() }
                        ]
                    },
                    Instruction::Loop{
                        position: Position { start: 5, end: 11 },
                        body: vec![
                            Instruction::Add { amount: 1, position: 6.into() },
                            Instruction::Loop{
                                position: Position { start: 7, end: 9 },
                                body: vec![
                                    Instruction::Output { position: 8.into() }
                                ]
                            },
                            Instruction::Add { amount: u8::MAX, position: 10.into() }
                        ]
                    },
                    Instruction::Add { amount: u8::MAX, position: 12.into() }
                ]
            }
        ]);
    }

    #[test]
    fn test_mismatched_brackets() {

        let prog = Cursor::new("[");
        assert!(parse(prog).is_err());

        let prog = Cursor::new("]");
        assert!(parse(prog).is_err());

        let prog = Cursor::new("[[]");
        assert!(parse(prog).is_err());

        let prog = Cursor::new("[][");
        assert!(parse(prog).is_err());

        let prog = Cursor::new("[[]");
        assert!(parse(prog).is_err());

        let prog = Cursor::new("[]]");
        assert!(parse(prog).is_err());

        let prog = Cursor::new("[[");
        assert!(parse(prog).is_err());

    }

}