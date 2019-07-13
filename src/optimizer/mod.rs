pub mod passes;

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
                passes.extend(DEFAULT_OPTIMIZATION_PASSES.iter().cloned());
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
        
        // Ideally, we would like to repeat the whole pipeline of passes
        // until we reach the fixed point, but this should be enough.
        for _ in 0..10 {
            for pass in &self.passes {
                accum = pass.run(accum);
            }
        }

        accum
    }

}

// Builds a static maps of all the passes
lazy_static! {

    /// [`HashMap`](std::collections::HashMap) containing all the registered optimization passes.
    pub static ref ALL_OPTIMIZATIONS: HashMap<&'static str, Arc<dyn Pass + Sync + Send>> = {
        use passes::*;
        let mut map: HashMap<_, Arc<dyn Pass + Sync + Send>> = HashMap::new();
        map.insert("clear-loops", Arc::new(ClearLoops));
        map.insert("mul-loops", Arc::new(MulLoops));
        map.insert("collapse-increments", Arc::new(CollapseIncrements));
        map.insert("dead-code", Arc::new(DeadCode));
        map
    };

    /// Order of the default optimizaiton passes.
    pub static ref DEFAULT_OPTIMIZATION_PASSES: Vec<Arc<dyn Pass + Sync + Send>> = vec![
        Arc::clone(&ALL_OPTIMIZATIONS["dead-code"]),
        Arc::clone(&ALL_OPTIMIZATIONS["collapse-increments"]),
        Arc::clone(&ALL_OPTIMIZATIONS["mul-loops"])

        // clear-loops is not included because it is strictly included by mul-loops
    ];

}