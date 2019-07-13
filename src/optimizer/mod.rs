use std::collections::HashMap;
use std::sync::Arc;
use crate::BrainfuckError;
use crate::parser::Instruction;

/// An optimization pass.
pub trait Pass {

    /// Name of the pass.
    fn name(&self) -> &str;

    /// Executes the pass on the given set of instructions.
    /// Returns the new set of optimized instructions.
    fn run(&self, instructions: Vec<Instruction>) -> Vec<Instruction>;

}

/// Brainfuck IR optimizer.
pub struct Optimizer {
    passes: Vec<Arc<dyn Pass + Sync + Send>>
}

impl Optimizer {

    /// Constructs a new optimizer with the given set of passes.
    pub fn with_passes(passes: Vec<Arc<dyn Pass + Sync + Send>>) -> Optimizer {
        Optimizer {
            passes
        }
    }

    /// Constructs a new optimizer with the given set of passes.
    /// The passes are specified as a comma-separated string of names
    pub fn with_passes_str(s: &str) -> Result<Optimizer, BrainfuckError> {

        let mut passes = Vec::new();

        match s {
            "none" => {
                // Do nothing, the vector of passes will be empty
            },
            "all" => {
                // All the passes
                passes.extend(ALL_OPTIMIZATIONS.values().cloned());
            },
            _ => {
                // Each pass is separated by `,`        
                for name in s.split(',') {
                    if let Some(arc) = ALL_OPTIMIZATIONS.get(name) {
                        passes.push(Arc::clone(arc));
                    } else {
                        return Err(BrainfuckError::UnknownOptimizationPass(name.to_owned()));
                    }
                }
            }
        }
        
        Ok(Optimizer {
            passes
        })
    }

    /// Returns a slice containing the passes configured for this oprimizer.
    pub fn passes(&self) -> &[Arc<dyn Pass + Sync + Send>] {
        &*self.passes
    }
    
    /// Runs all the passes on the given set of instructions
    pub fn run(&self, instructions: Vec<Instruction>) -> Vec<Instruction> {
        let mut accum = instructions;
        for pass in &self.passes {
            accum = pass.run(accum);
        }
        accum
    }

}

// Links to the modules of all the passes
pub mod collapse_increments;
pub mod dead_code;

lazy_static! {
    pub static ref ALL_OPTIMIZATIONS: HashMap<&'static str, Arc<dyn Pass + Sync + Send>> = {
        let mut map: HashMap<_, Arc<dyn Pass + Sync + Send>> = HashMap::new();
        map.insert("collapse-increments", Arc::new(collapse_increments::CollapseIncrements));
        map.insert("dead-code", Arc::new(dead_code::DeadCode));
        map
    };
}