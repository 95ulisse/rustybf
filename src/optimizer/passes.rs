use itertools::Itertools;
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
                }

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
        // it means the the current cell value is 0, thus the next loop will never be executed
        .coalesce(|a, b| {
            match (a, b) {
                (a @ Loop { .. }, Loop { .. }) => Ok(a),
                (a, b) => Err((a, b))
            }
        })

        // For a similar reason, all loops at the beginning of the program are dead code,
        // since all the cells are initialized as zero.
        .skip_while(|i| match i {
            Loop { .. } => true,
            _ => false
        })

        .collect()
    }

}