use crate::parser::Instruction;
use crate::optimizer::Pass;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollapseIncrements;

impl Pass for CollapseIncrements {

    fn name(&self) -> &str {
        "collapse-increments"
    }

    fn run(&self, instructions: &[Instruction]) -> Vec<Instruction> {
        Vec::from(instructions)
    }

}