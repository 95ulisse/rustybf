use std::io::{self, Read};
use crate::BrainfuckError;

/// A single Brainfuck instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Add,
    Sub,
    Right,
    Left,
    Input,
    Output,
    Loop(Vec<Instruction>)
}

fn parse_impl<I>(it: I, level: &mut u32) -> Result<(Vec<Instruction>, I), BrainfuckError>
    where I: Iterator<Item = Result<u8, io::Error>>
{
    let mut v = Vec::new();
    let mut it = it;
    while let Some(b) = it.next() {
        match b {
            Err(e) => return Err(BrainfuckError::IoError(e)),
            Ok(b'>') => v.push(Instruction::Right),
            Ok(b'<') => v.push(Instruction::Left),
            Ok(b'+') => v.push(Instruction::Add),
            Ok(b'-') => v.push(Instruction::Sub),
            Ok(b'.') => v.push(Instruction::Output),
            Ok(b',') => v.push(Instruction::Input),
            Ok(b'[') => {
                *level += 1;
                match parse_impl(it, level) {
                    Err(e) => return Err(e),
                    Ok((inner, it2)) => {
                        it = it2;
                        v.push(Instruction::Loop(inner));
                    }
                }
            }
            Ok(b']') => {
                if *level == 0 {
                    return Err(BrainfuckError::MismatchedBrackets);
                } else {
                    *level -= 1;
                    return Ok((v, it));
                }
            },
            Ok(_) => { /* Ignore every other char */ }
        }
    }
    Ok((v, it))
}

/// Parses a Brainfuck program from the given stream.
pub fn parse(r: impl Read) -> Result<Vec<Instruction>, BrainfuckError> {
    let mut level = 0u32;
    parse_impl(r.bytes(), &mut level)
        .and_then(|(v, _)| {
            if level > 0 {
                Err(BrainfuckError::MismatchedBrackets)
            } else {
                Ok(v)
            }
        })
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
            Instruction::Add,
            Instruction::Sub,
            Instruction::Right,
            Instruction::Left,
            Instruction::Output,
            Instruction::Input
        ]);
    }

    #[test]
    fn test_empty_loop() {
        let prog = Cursor::new("[]");
        assert_eq!(parse(prog).unwrap(), vec![
            Instruction::Loop(vec![])
        ]);
    }

    #[test]
    fn test_empty_nested_loop() {
        let prog = Cursor::new("[+[,][+[.]-]-]");
        assert_eq!(parse(prog).unwrap(), vec![
            Instruction::Loop(vec![
                Instruction::Add,
                Instruction::Loop(vec![
                    Instruction::Input
                ]),
                Instruction::Loop(vec![
                    Instruction::Add,
                    Instruction::Loop(vec![
                        Instruction::Output
                    ]),
                    Instruction::Sub
                ]),
                Instruction::Sub
            ])
        ]);
    }

    #[test]
    fn test_mismatched_brackets() {

        fn assert_mismatched(r: Result<Vec<Instruction>, BrainfuckError>) {
            match r {
                Err(BrainfuckError::MismatchedBrackets) => {},
                _ => panic!("Expected mismatched brackets error. Got: {:?}", r)
            }
        }

        let prog = Cursor::new("[");
        assert_mismatched(parse(prog));

        let prog = Cursor::new("]");
        assert_mismatched(parse(prog));

        let prog = Cursor::new("[[]");
        assert_mismatched(parse(prog));

        let prog = Cursor::new("[][");
        assert_mismatched(parse(prog));

        let prog = Cursor::new("[[]");
        assert_mismatched(parse(prog));

        let prog = Cursor::new("[]]");
        assert_mismatched(parse(prog));

        let prog = Cursor::new("[[");
        assert_mismatched(parse(prog));

    }

}