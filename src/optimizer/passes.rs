use std::collections::HashMap;
use std::num::Wrapping;
use std::u8;
use itertools::{Itertools, Either};
use crate::parser::Instruction;
use crate::optimizer::Pass;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollapseIncrements;

impl Pass for CollapseIncrements {

    fn name(&self) -> &str {
        "collapse-increments"
    }

    fn run(&self, instructions: Vec<Instruction>) -> Vec<Instruction> {
        use Instruction::*;
        instructions.into_iter().coalesce(|a, b| {
            match (a, b) {

                // Merge consecutive adds together
                (Add { amount: x, position: posa }, Add { amount: y, position: posb }) => {
                    Ok(Add {
                        amount: x + y,
                        position: posa.merge(posb)
                    })
                },

                // Merge consecutive moves
                (Move { offset: x, position: posa }, Move { offset: y, position: posb }) => {
                    Ok(Move {
                        offset: x + y,
                        position: posa.merge(posb)
                    })
                },

                // Merge also the clears
                (Clear { position: posa }, Clear { position: posb }) => {
                    Ok(Clear {
                        position: posa.merge(posb)
                    })
                },

                (a, b) => Err((a, b))

            }
        })

        // Recurse inside loops
        .map(|i| match i {
            Loop { body, position } => {
                Loop {
                    body: CollapseIncrements.run(body),
                    position
                }
            },
            _ => i
        })

        .collect()
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadCode;

impl Pass for DeadCode {

    fn name(&self) -> &str {
        "dead-code"
    }

    fn run(&self, instructions: Vec<Instruction>) -> Vec<Instruction> {
        remove_dead_code_inner(instructions, true)
    }

}

fn remove_dead_code_inner(instructions: Vec<Instruction>, skip_initial: bool) -> Vec<Instruction> {
    use Instruction::*;
        
    // First of all, remove null increments
    instructions.into_iter().filter(|i| match i {
        Add { amount: Wrapping(0), .. } |
        Move { offset: 0, .. } => false,
        _ => true
    })

    // Loops at the beginning of the program are dead code,
    // since all the cells are initialized as zero.
    .skip_while(|i| i.is_loop() && skip_initial)

    // Remove consecutive loops. When we have two consecutive loops,
    // the second one is dead code because if the previous one exited,
    // it means the the current cell value is 0, thus the next loop will never be executed.
    .coalesce(|a, b| {
        if a.clears_current_cell() && b.is_loop() {
            Ok(a)
        } else {
            Err((a, b))
        }
    })

    // Recurse inside surviving loops
    .map(|i| match i {
        Loop { body, position } => {
            Loop {
                body: remove_dead_code_inner(body, false),
                position
            }
        },
        _ => i
    })

    .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearLoops;

impl Pass for ClearLoops {

    fn name(&self) -> &str {
        "clear-loops"
    }

    fn run(&self, instructions: Vec<Instruction>) -> Vec<Instruction> {
        use Instruction::*;
        instructions.into_iter()
        
        // `[-]` is a very common idiom to clear the current cell.
        .map(|i| match &i {
            Loop { ref body, position } => {
                match body.as_slice() {
                    [ Add { amount: Wrapping(u8::MAX), .. } ] => {
                        Clear { position: *position }
                    },
                    _ => i
                }
            },
            _ => i
        })

        // Recurse inside surviving loops
        .map(|i| match i {
            Loop { body, position } => {
                Loop {
                    body: ClearLoops.run(body),
                    position
                }
            },
            _ => i
        })

        .collect()
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MulLoops;

impl Pass for MulLoops {

    fn name(&self) -> &str {
        "mul-loops"
    }

    fn run(&self, instructions: Vec<Instruction>) -> Vec<Instruction> {
        use Instruction::*;
        instructions.into_iter()
        
        // Check if each loop is a multiplication
        .flat_map(|i| match i {
            Loop { ref body, position } => {
                if let Some(multiplications) = recognize_mul_loop(body) {

                    // Replace each multiplication with the corresponding Mul and end with a Clear
                    Either::Left(
                        multiplications.into_iter()
                        .map(move |(offset, amount)| Instruction::Mul { offset, amount, position })
                        .chain(::std::iter::once(Instruction::Clear { position }))
                    )

                } else {
                    Either::Right(::std::iter::once(i))
                }
            },
            _ => Either::Right(::std::iter::once(i))
        })

        // Recurse inside surviving loops
        .map(|i| match i {
            Loop { body, position } => {
                Loop {
                    body: MulLoops.run(body),
                    position
                }
            },
            _ => i
        })

        .collect()
    }

}

/// Recognizes if the body of a loop is a multiplication loop.
/// The returned value is a map recording the offsets and their multiplicative factors, i.e.
/// if the mapping `i => x` is in the returned map, then the cell at offset `i` from the current one
/// will be added a value equal to the current cell times `x`.
fn recognize_mul_loop(instructions: &[Instruction]) -> Option<HashMap<isize, Wrapping<u8>>> {
    
    // Compute a map of all the cells modified by the instructions
    let mut res: HashMap<isize, Wrapping<u8>> = HashMap::new();
    let mut offset: isize = 0;
    for i in instructions {
        match i {

            Instruction::Move { offset: off, .. } => {
                offset += off;
            },

            Instruction::Add { amount, .. } => {
                *res.entry(offset).or_default() += *amount;
            },

            _ => {
                // Any other instruction means that this is not a multiplication loop
                return None;
            }

        }
    }

    // If the number of lefts and rights were not balanced,
    // we ended up in a cell different from the one we started,
    // so this is not a mul
    if offset != 0 {
        return None;
    }
    
    // The loop must decrement the first cell by exactly 1 each iteration
    match res.get(&0) {
        Some(Wrapping(u8::MAX)) => {
            // Remove the 0 from the map because it's implicit
            res.remove(&0);
        },
        _ => return None
    }

    Some(res)

}



#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::parser::parse;

    macro_rules! map(
        { } => { ::std::collections::HashMap::new() };
        { $($key:expr => $value:expr),+ } => {
            {
                let mut m = ::std::collections::HashMap::new();
                $(
                    m.insert($key, Wrapping($value));
                )+
                m
            }
        };
    );

    fn p(s: &str) -> Vec<Instruction> {
        parse(Cursor::new(s)).unwrap()
    }

    #[test]
    fn test_recognize_mul_loop() {

        // Empty loop
        assert_eq!(recognize_mul_loop(&p("-")).unwrap(), map! {});

        // Loop with single multiplication
        assert_eq!(recognize_mul_loop(&p("->+<")).unwrap(), map! {
            1 => 1
        });
        assert_eq!(recognize_mul_loop(&p("->++<")).unwrap(), map! {
            1 => 2
        });

        // Loop with more than one single multiplication
        assert_eq!(recognize_mul_loop(&p("->+>+<<")).unwrap(), map! {
            1 => 1,
            2 => 1
        });
        assert_eq!(recognize_mul_loop(&p("->++>+++<<")).unwrap(), map! {
            1 => 2,
            2 => 3
        });

        // Negative offsets
        assert_eq!(recognize_mul_loop(&p("-<+>")).unwrap(), map! {
            -1 => 1
        });
        assert_eq!(recognize_mul_loop(&p("-<+>>+<")).unwrap(), map! {
            -1 => 1,
            1 => 1
        });

        // Strange loops with interleaving sums
        assert_eq!(recognize_mul_loop(&p("->>++<++++>+>++<<<<-->")).unwrap(), map! {
            -1 => 254 /* = -2 */,
            1 => 4,
            2 => 3,
            3 => 2
        });

        // Loops must not start with a `-`
        assert_eq!(recognize_mul_loop(&p(">+<->+<")).unwrap(), map! {
            1 => 2
        });

        // Now a couple of tests on invalid loops
        assert!(recognize_mul_loop(&p("")).is_none());
        assert!(recognize_mul_loop(&p("+")).is_none());
        assert!(recognize_mul_loop(&p("--")).is_none());
        assert!(recognize_mul_loop(&p("->")).is_none());
        assert!(recognize_mul_loop(&p("-<")).is_none());
        assert!(recognize_mul_loop(&p("->+<+")).is_none());

    }

}