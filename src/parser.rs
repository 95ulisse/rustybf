use std::io::Read;
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

/// A single Brainfuck instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Add {
        amount: u8,
        position: Position
    },
    Sub {
        amount: u8,
        position: Position
    },
    Right {
        position: Position
    },
    Left{
        position: Position
    },
    Input{
        position: Position
    },
    Output{
        position: Position
    },
    Loop{
        body: Vec<Instruction>,
        position: Position
    }
}

impl Instruction {
    
    /// Returns the position of this instruction in the source code.
    pub fn position(&self) -> Position {
        match *self {
             Instruction::Add { position, .. } => position,
             Instruction::Sub { position, .. } => position,
             Instruction::Right { position, .. } => position,
             Instruction::Left { position, .. } => position,
             Instruction::Input { position, .. } => position,
             Instruction::Output { position, .. } => position,
             Instruction::Loop { position, .. } => position
        }
    }

}

/// Parses a Brainfuck program from the given stream.
pub fn parse(r: impl Read) -> Result<Vec<Instruction>, BrainfuckError> {

    let mut instructions: Vec<Instruction> = Vec::new();
    let mut stack: Vec<(Vec<Instruction>, usize)> = Vec::new();

    for (index, res) in r.bytes().enumerate() {
        match res {
            Err(e) => return Err(BrainfuckError::IoError(e)),
            Ok(b'>') => instructions.push(Instruction::Right  { position: index.into() }),
            Ok(b'<') => instructions.push(Instruction::Left   { position: index.into() }),
            Ok(b'+') => instructions.push(Instruction::Add    { position: index.into(), amount: 1  }),
            Ok(b'-') => instructions.push(Instruction::Sub    { position: index.into(), amount: 1 }),
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
            Instruction::Sub { amount: 1, position: 1.into() },
            Instruction::Right { position: 2.into() },
            Instruction::Left { position: 3.into() },
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
                            Instruction::Sub { amount: 1, position: 10.into() }
                        ]
                    },
                    Instruction::Sub { amount: 1, position: 12.into() }
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