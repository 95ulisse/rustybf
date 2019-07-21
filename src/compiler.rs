use std::cell::RefCell;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use inkwell::{AddressSpace, OptimizationLevel};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::{Module, Linkage};
use inkwell::targets::{CodeModel, RelocMode, FileType, Target, TargetMachine, InitializationConfig};
use tempfile::NamedTempFile;
use crate::{BrainfuckError, Instruction};

/// Compiler from Brainfuck to native code.
pub struct Compiler {
    context: Context,
    module: Module,
    builder: Builder,
    optimization_level: OptimizationLevel
}

impl Compiler {

    /// Creates a new compiler with the given optimization level.
    /// For more information about optimization levels, refer to the LLVM documentation.    
    pub fn new(optimization_level: u32) -> Compiler {
        
        // Match the optimization level to one of those available for LLVM
        let opt = match optimization_level {
            0     => OptimizationLevel::None,
            1     => OptimizationLevel::Less,
            2     => OptimizationLevel::Default,
            3 | _ => OptimizationLevel::Aggressive
        };

        let context = Context::create();
        let module = context.create_module("brainfuck");
        let builder = context.create_builder();

        let void_type = context.void_type();
        let i8_ptr_type = context.i8_type().ptr_type(AddressSpace::Generic);
        let i32_type = context.i32_type();

        // Define the two extern functions that will be needed to implement I/O.
        // Since the output program will be linked against libc, we can use `putchar` and `getchar`.
        let getchar_type = i32_type.fn_type(&[], false);
        let putchar_type = i32_type.fn_type(&[i32_type.into()], false);
        module.add_function("getchar", getchar_type, Some(Linkage::External));
        module.add_function("putchar", putchar_type, Some(Linkage::External));

        // Same reason, declare memory management functions `calloc` and `free`
        // to manage the tape
        let calloc_type = i8_ptr_type.fn_type(&[i32_type.into(), i32_type.into()], false);
        let free_type = void_type.fn_type(&[i8_ptr_type.into()], false);
        module.add_function("calloc", calloc_type, Some(Linkage::External));
        module.add_function("free", free_type, Some(Linkage::External));

        // Create a `main` function
        let fn_type = context.void_type().fn_type(&[], false);
        let main_function = module.add_function("main", fn_type, None);

        // Create a builder positioned at the body of the main function
        let entry_block = context.append_basic_block(&main_function, "entry");
        builder.position_at_end(&entry_block);

        // Return a constant
        let value = i32_type.const_int(42, false);
        builder.build_return(Some(&value));

        Compiler {
            context,
            module,
            builder,
            optimization_level: opt
        }
    }

    /// Compiles the given instructions. This method can be called multiple times,
    /// allowing to compile instructions in a streaming fashion.
    /// To conclude the compilation, call the `finish()` method.
    pub fn compile_instructions(self, _instructions: &[Instruction]) -> Self {
        self
    }

    /// Finishes the streaming compilation.
    pub fn finish(self) -> CompiledProgram {
        CompiledProgram {
            context: self.context,
            module: self.module,
            execution_engine: RefCell::new(None),
            builder: self.builder,
            optimization_level: self.optimization_level
        }
    }

    /// Dumps the currently compiled instructions as LLVM IR to the given stream.
    pub fn dump(&self, target: &mut impl Write) -> Result<(), BrainfuckError> {
        let s = self.module.print_to_string();
        writeln!(target, "{}", s.to_string())?;
        Ok(())
    }

}

/// Compiled Brainfuck program, ready to be JITed or saved to disk.
pub struct CompiledProgram {
    context: Context,
    module: Module,
    execution_engine: RefCell<Option<ExecutionEngine>>,
    builder: Builder,
    optimization_level: OptimizationLevel
}

impl CompiledProgram {

    /// Executes the compiled program.
    pub fn run(&self) {

        // This is the type of the main function we defined in `new()`
        type MainFn = unsafe extern "C" fn();

        // Initialize the execution engine if not done yet
        if self.execution_engine.borrow().is_none() {
            let engine = self.module.create_jit_execution_engine(self.optimization_level).expect("Cannot create JIT engine");
            *self.execution_engine.borrow_mut() = Some(engine);
        }

        unsafe {
            // Compile and invoke the entry point
            let engine = self.execution_engine.borrow();
            let main = engine.as_ref().unwrap().get_function::<MainFn>("main").expect("Cannot JIT compile entry point");
            main.call();
        }

    }

    /// Saves the compiled program on disk as an object file.
    pub fn save_object<P: AsRef<Path>>(&self, path: P) -> Result<(), BrainfuckError> {
        
        Target::initialize_all(&InitializationConfig::default());

        // Prepare a TargetMachine targeting the current host
        let triple = TargetMachine::get_default_triple().to_string();
        let target = Target::from_triple(&triple).map_err(|e| format!("Cannot create Target: {}", e.to_string()))?;
        let target_machine = target.create_target_machine(
            &triple,
            &TargetMachine::get_host_cpu_name().to_string(),
            &TargetMachine::get_host_cpu_features().to_string(),
            self.optimization_level,
            RelocMode::Default,
            CodeModel::Default
        ).ok_or("Cannot create TargetMachine")?;

        // Save to file
        target_machine.write_to_file(&self.module, FileType::Object, path.as_ref())
            .map_err(|e| format!("Failed to write object file: {}", e.to_string()))?;

        Ok(())
    }

    /// Saves the compiled program on disk as an executable.
    /// 
    /// The program is first compiled as an object file in a temporary location,
    /// then it is linked using `clang`.
    pub fn save_executable<P: AsRef<Path>>(&self, path: P) -> Result<(), BrainfuckError> {

        // Compile the program to a temporary location
        let file = NamedTempFile::new()?;
        self.save_object(file.path())?;

        // Use `clang` to link the object file
        let status = Command::new("clang")
            .args(&[ file.path(), &Path::new("-o"), path.as_ref() ])
            .status()
            .expect("Failed to execute process");

        if !status.success() {
            Err("Cannot link using clang. Be sure that clang is installed and available in $PATH.".into())
        } else {
            Ok(())
        }
    }

    /// Dumps the currently compiled instructions as LLVM IR to the given stream.
    pub fn dump(&self, target: &mut impl Write) -> Result<(), BrainfuckError> {
        let s = self.module.print_to_string();
        writeln!(target, "{}", s.to_string())?;
        Ok(())
    }

}