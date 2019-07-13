use itertools::Itertools;
use crate::parser::Instruction;
use crate::optimizer::Pass;

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