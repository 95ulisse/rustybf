use std::collections::HashMap;
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
                        amount: x.wrapping_add(y),
                        position: posa.merge(posb)
                    })
                },

                // Merge consecutive lefts together
                (Left { amount: x, position: posa }, Left { amount: y, position: posb }) => {
                    Ok(Left {
                        amount: x.wrapping_add(y),
                        position: posa.merge(posb)
                    })
                },

                // Merge consecutive rights together
                (Right { amount: x, position: posa }, Right { amount: y, position: posb }) => {
                    Ok(Right {
                        amount: x.wrapping_add(y),
                        position: posa.merge(posb)
                    })
                },

                // We can also merge alternating lefts and rights
                (Left  { amount: l, position: posa }, Right { amount: r, position: posb }) |
                (Right { amount: r, position: posa }, Left  { amount: l, position: posb }) => {
                    if l >= r {
                        Ok(Left {
                            amount: l - r,
                            position: posa.merge(posb)
                        })
                    } else {
                        Ok(Right {
                            amount: r - l,
                            position: posa.merge(posb)
                        })
                    }
                },

                // Merge also the clears
                (Clear { position: posa }, Clear { position: posb }) => {
                    Ok(Clear {
                        position: posa.merge(posb)
                    })
                },

                // Loops must be optimized too
                (Loop { body, position }, other) => {
                    Err((Loop {
                        body: CollapseIncrements.run(body),
                        position
                    }, other))
                }

                (a, b) => Err((a, b))

            }
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
        use Instruction::*;
        
        // First of all, remove null increments
        instructions.into_iter().filter(|i| match i {
            Add { amount: 0, .. } |
            Right { amount: 0, .. } |
            Left { amount: 0, .. } => false,
            _ => true
        })

        // Remove consecutive loops. When we have two consecutive loops,
        // the second one is dead code because if the previous one exited,
        // it means the the current cell value is 0, thus the next loop will never be executed.
        // The `Clear` instruction is just a collapsed loop, so it counts too.
        .coalesce(|a, b| {
            match (a, b) {
                (a @ Loop { .. }, Loop { .. }) |
                (a @ Loop { .. }, Clear { .. }) => Ok(a),

                (a, b) => Err((a, b))
            }
        })

        // For a similar reason, all loops at the beginning of the program are dead code,
        // since all the cells are initialized as zero.
        .skip_while(|i| match i {
            Loop { .. } => true,
            _ => false
        })

        // Recurse inside surviving loops
        .map(|i| match i {
            Loop { body, position } => {
                Loop {
                    body: DeadCode.run(body),
                    position
                }
            },
            _ => i
        })

        .collect()
    }

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
                    [ Add { amount: 255, .. } ] => {
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
/// will be multiplied by factor `x`.
fn recognize_mul_loop(instructions: &[Instruction]) -> Option<HashMap<isize, u8>> {
    
    // First instruction must be a `-`
    if instructions.is_empty() {
        return None;
    }
    match instructions.first().unwrap() {
        Instruction::Add { amount: 255, .. } => {},
        _ => return None
    }

    // Validate the rest of the instructions
    let mut res: HashMap<isize, u8> = HashMap::new();
    let mut offset: isize = 0;
    for i in &instructions[1..] {
        match i {

            Instruction::Left { amount, .. } => {
                offset -= *amount as isize;
            },

            Instruction::Right { amount, .. } => {
                offset += *amount as isize;
            },

            Instruction::Add { amount, .. } => {

                // If we are incrementing the cell at offset 0, we are changing the iteration
                // counter, thus this is not a mul loop
                if offset == 0 {
                    return None;
                }

                let x: &mut _ = res.entry(offset).or_default();
                *x = x.wrapping_add(*amount);

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
        None
    } else {
        Some(res)
    }

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
                    m.insert($key, $value);
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

        // Now a couple of tests on invalid loops
        assert!(recognize_mul_loop(&p("")).is_none());
        assert!(recognize_mul_loop(&p("+")).is_none());
        assert!(recognize_mul_loop(&p("--")).is_none());
        assert!(recognize_mul_loop(&p("->")).is_none());
        assert!(recognize_mul_loop(&p("-<")).is_none());
        assert!(recognize_mul_loop(&p("->+<+")).is_none());

    }

}