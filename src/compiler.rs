use std::cell::RefCell;
use std::io::{Read, Write};
use std::mem;
use std::num::Wrapping;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
use inkwell::{AddressSpace, OptimizationLevel, IntPredicate};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::{Module, Linkage};
use inkwell::targets::{CodeModel, RelocMode, FileType, Target, TargetMachine, InitializationConfig};
use inkwell::values::{BasicValueEnum, PointerValue, FunctionValue};
use tempfile::NamedTempFile;
use crate::{BrainfuckError, Instruction};

/// Configuration for the input of a JITed program.
pub enum InputTarget {
    /// Use stdin.
    Stdio,
    /// Use the given stream.
    Custom(Rc<RefCell<dyn Read>>)
}

/// Configuration for the output of a JITed program.
pub enum OutputTarget {
    /// Use stdout.
    Stdio,
    /// Use the given stream.
    Custom(Rc<RefCell<dyn Write>>)
}

struct IoTarget {
    input: InputTarget,
    output: OutputTarget
}

/// Compiler from Brainfuck to native code.
pub struct Compiler {
    context: Context,
    module: Module,
    builder: Builder,
    optimization_level: OptimizationLevel,
    io: Box<IoTarget>,

    // A couple of useful values inside the emitted function
    tape: BasicValueEnum,
    ptr: PointerValue
}

impl Compiler {

    /// Creates a new compiler with the given optimization level.
    /// For more information about optimization levels, refer to the LLVM documentation.    
    pub fn new(optimization_level: u32) -> Compiler {
        Compiler::new_with_io(optimization_level, InputTarget::Stdio, OutputTarget::Stdio)
    }

    /// Creates a new compiler with the given optimization level and custom I/O.
    /// For more information about optimization levels, refer to the LLVM documentation.    
    pub fn new_with_io(optimization_level: u32, input: InputTarget, output: OutputTarget) -> Compiler {
        
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

        // If we need custom I/O, redefine `getchar` and `putchar` to intercept the calls.
        // In case of stdio instead, use the ones from libc.
        let io_target = Box::new(IoTarget { input, output });
        let getchar_type = i32_type.fn_type(&[], false);
        let putchar_type = i32_type.fn_type(&[i32_type.into()], false);
        match io_target.input {
            InputTarget::Stdio => {
                module.add_function("getchar", getchar_type, Some(Linkage::External));
            },
            InputTarget::Custom(_) => {
                let f = module.add_function("getchar", getchar_type, None);
                let entry_block = context.append_basic_block(&f, "entry");
                builder.position_at_end(&entry_block);
                emit_getchar_interceptor(&context, &builder, &*io_target);
            }
        }
        match io_target.output {
            OutputTarget::Stdio => {
                module.add_function("putchar", putchar_type, Some(Linkage::External));
            },
            OutputTarget::Custom(_) => {
                let f = module.add_function("putchar", putchar_type, None);
                let entry_block = context.append_basic_block(&f, "entry");
                builder.position_at_end(&entry_block);
                emit_putchar_interceptor(&context, &f, &builder, &*io_target);
            }
        }

        // Same reason, declare memory management functions `calloc` and `free`
        // to manage the tape
        let calloc_type = i8_ptr_type.fn_type(&[i32_type.into(), i32_type.into()], false);
        let free_type = void_type.fn_type(&[i8_ptr_type.into()], false);
        let calloc_fn = module.add_function("calloc", calloc_type, Some(Linkage::External));
        module.add_function("free", free_type, Some(Linkage::External));

        // Create a `main` function
        let fn_type = context.void_type().fn_type(&[], false);
        let main_function = module.add_function("main", fn_type, None);

        // Create a builder positioned at the body of the main function
        let entry_block = context.append_basic_block(&main_function, "entry");
        builder.position_at_end(&entry_block);

        // First things first: reserve space for the local variables
        let ptr = builder.build_alloca(i8_ptr_type, "ptr");

        // Emit runtime setup: use `calloc` to create space for 30.000 cells
        let tape =
            builder.build_call(
                calloc_fn,
                &[
                    i32_type.const_int(30_000, false).into(),
                    i32_type.const_int(1, false).into()
                ],
                "tape"
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        // Allocate the variable that will be the pointer moved on the tape
        builder.build_store(ptr, tape);

        Compiler {
            context,
            module,
            builder,
            optimization_level: opt,
            io: io_target,
            tape,
            ptr
        }
    }

    /// Compiles the given instructions. This method can be called multiple times,
    /// allowing to compile instructions in a streaming fashion.
    /// To conclude the compilation, call the `finish()` method.
    pub fn compile_instructions(mut self, instructions: &[Instruction]) -> Self {
        let i8_type = self.context.i8_type();
        let i32_type = self.context.i32_type();
        let putchar_fn = self.module.get_function("putchar").unwrap();
        let getchar_fn = self.module.get_function("getchar").unwrap();

        for instruction in instructions {
            match instruction {
                
                Instruction::Add { amount: Wrapping(amount), .. } => {
                    // Fetch the value of the cell pointed from `ptr`, increment it and store it back
                    let ptr = self.builder.build_load(self.ptr, "ptr");
                    let value = self.builder.build_load(ptr.into_pointer_value(), "value");
                    let value = self.builder.build_int_add(value.into_int_value(), i8_type.const_int((*amount).into(), false), "value");
                    self.builder.build_store(ptr.into_pointer_value(), value);
                },
                
                Instruction::Move { offset, .. } => {
                    // Load the cell pointer, add the offset, store it back on the stack
                    let ptr = self.builder.build_load(self.ptr, "ptr");
                    let ptr = unsafe { self.builder.build_in_bounds_gep(ptr.into_pointer_value(), &[ i32_type.const_int(*offset as u64, false) ], "ptr") };
                    self.builder.build_store(self.ptr, ptr);
                },
                
                Instruction::Input { .. } => {
                    // Call `getchar`, truncate the result and store it into the current cell
                    let ptr = self.builder.build_load(self.ptr, "ptr");
                    let value = self.builder.build_call(getchar_fn, &[], "input_value").try_as_basic_value().left().unwrap();
                    let value = self.builder.build_int_truncate(value.into_int_value(), i8_type, "input_value");
                    self.builder.build_store(ptr.into_pointer_value(), value);
                },
                
                Instruction::Output { .. } => {
                    // Fetch the current cell and call `putchar`
                    let ptr = self.builder.build_load(self.ptr, "ptr");
                    let value = self.builder.build_load(ptr.into_pointer_value(), "value");
                    self.builder.build_call(putchar_fn, &[
                        self.builder.build_int_s_extend(value.into_int_value(), i32_type, "").into()
                    ], "");
                },
                
                Instruction::Loop { body, .. } => {
                    // The idea is having three blocks like this:
                    //
                    // ```
                    //     br loop_guard
                    //
                    // loop_guard:
                    //     <load *ptr>
                    //     <jump to loop_body if *ptr != 0, to loop_end otherwise>
                    //
                    // loop_body:
                    //     <loop body>
                    //     br loop_guard
                    //
                    // loop_end:
                    //     <continue generation from here>
                    // ```
                    //
                    // This is equivalent to:
                    // while (*ptr != 0) { ... }

                    // Start by creating the three blocks
                    let main_function = self.builder.get_insert_block().unwrap().get_parent().unwrap();
                    let loop_guard = self.context.append_basic_block(&main_function, "loop_guard");
                    let loop_body = self.context.append_basic_block(&main_function, "loop_body");
                    let loop_end = self.context.append_basic_block(&main_function, "loop_end");

                    // Jump unconditionally to the loop guard
                    self.builder.build_unconditional_branch(&loop_guard);

                    // Emit the loop guard
                    self.builder.position_at_end(&loop_guard);
                    let ptr = self.builder.build_load(self.ptr, "ptr");
                    let value = self.builder.build_load(ptr.into_pointer_value(), "value");
                    let guard_value = self.builder.build_int_compare(IntPredicate::EQ, value.into_int_value(), i8_type.const_int(0, false), "guard_value");
                    self.builder.build_conditional_branch(guard_value, &loop_end, &loop_body);

                    // Emit the loop body
                    self.builder.position_at_end(&loop_body);
                    self = self.compile_instructions(&body);
                    self.builder.build_unconditional_branch(&loop_guard);

                    // Position the builder at the end of the loop and let compilation continue from there
                    self.builder.position_at_end(&loop_end);
                    
                },
                
                Instruction::Clear { .. } => {
                    // Store a 0 in the cell pointed by `ptr`
                    let ptr = self.builder.build_load(self.ptr, "ptr");
                    self.builder.build_store(ptr.into_pointer_value(), i8_type.const_int(0, false));
                },
                
                Instruction::Mul { amount: Wrapping(amount), offset, .. } => {
                    // Basically build the equivalent of:
                    // *(ptr + offset) += *ptr * amount
                    let ptr = self.builder.build_load(self.ptr, "ptr");
                    let ptr_value = self.builder.build_load(ptr.into_pointer_value(), "ptr_value");
                    let ptr_value = self.builder.build_int_mul(ptr_value.into_int_value(), i8_type.const_int((*amount).into(), false), "ptr_value");
                    let target = unsafe { self.builder.build_in_bounds_gep(ptr.into_pointer_value(), &[ i32_type.const_int(*offset as u64, false) ], "target") };
                    let target_value = self.builder.build_load(target, "target_value");
                    let final_value = self.builder.build_int_add(ptr_value, target_value.into_int_value(), "final_value");
                    self.builder.build_store(target, final_value);
                }

            }
        }

        self
    }

    /// Finishes the streaming compilation.
    pub fn finish(self) -> CompiledProgram {

        // Finish the main function by calling `free()` on the tape
        let free_fn = self.module.get_function("free").unwrap();
        self.builder.build_call(free_fn, &[ self.tape ], "");

        // Emit a return
        let i32_type = self.context.i32_type();
        self.builder.build_return(Some(&i32_type.const_int(0, false)));

        CompiledProgram {
            module: self.module,
            execution_engine: RefCell::new(None),
            optimization_level: self.optimization_level,
            io: self.io
        }

    }

    /// Dumps the currently compiled instructions as LLVM IR to the given stream.
    pub fn dump(&self, target: &mut impl Write) -> Result<(), BrainfuckError> {
        let s = self.module.print_to_string();
        writeln!(target, "{}", s.to_string())?;
        Ok(())
    }

}

fn emit_getchar_interceptor(context: &Context, builder: &Builder, data: *const IoTarget) {
    
    // Declare the types we are going to need
    let i8_ptr_type = context.i8_type().ptr_type(AddressSpace::Generic);
    let i32_type = context.i32_type();
    let i64_type = context.i64_type();
    let interceptor_type = i32_type.fn_type(&[ i8_ptr_type.into() ], false);
    let interceptor_ptr_type = interceptor_type.ptr_type(AddressSpace::Generic);

    // Load the function address
    let function_address_int = i64_type.const_int(getchar_interceptor as u64, false);
    let function_address_ptr = builder.build_int_to_ptr(function_address_int, interceptor_ptr_type, "function_pointer");

    // Load the data context
    let data_address_int = i64_type.const_int(unsafe { mem::transmute(data) }, false);
    let data_address_ptr = builder.build_int_to_ptr(data_address_int, i8_ptr_type, "context_pointer");
    
    // Emit the call
    let ret = builder.build_call(function_address_ptr, &[ data_address_ptr.into() ], "")
        .try_as_basic_value()
        .left()
        .unwrap();
    builder.build_return(Some(&ret));

}

fn emit_putchar_interceptor(context: &Context, function: &FunctionValue, builder: &Builder, data: *const IoTarget) {

    // Declare the types we are going to need
    let i8_ptr_type = context.i8_type().ptr_type(AddressSpace::Generic);
    let i32_type = context.i32_type();
    let i64_type = context.i64_type();
    let interceptor_type = i32_type.fn_type(&[ i8_ptr_type.into(), i32_type.into() ], false);
    let interceptor_ptr_type = interceptor_type.ptr_type(AddressSpace::Generic);

    // Load the function address
    let function_address_int = i64_type.const_int(putchar_interceptor as u64, false);
    let function_address_ptr = builder.build_int_to_ptr(function_address_int, interceptor_ptr_type, "function_pointer");

    // Load the data context
    let data_address_int = i64_type.const_int(unsafe { mem::transmute(data) }, false);
    let data_address_ptr = builder.build_int_to_ptr(data_address_int, i8_ptr_type, "context_pointer");
    
    // Emit the call
    let ret =
        builder.build_call(
            function_address_ptr,
            &[
                data_address_ptr.into(),
                function.get_nth_param(0).unwrap()
            ],
            ""
        )
        .try_as_basic_value()
        .left()
        .unwrap();
    builder.build_return(Some(&ret));

}

/// Callback invoked during the execution of the Brainfuck program to intercept the input command `,`.
extern "C" fn getchar_interceptor(data: *const IoTarget) -> i32 {

    // Read a single byte from the input stream
    let data = unsafe { &*data };
    match data.input {
        InputTarget::Custom(ref r) => {
            let mut buf = [ 0u8 ];
            r.borrow_mut()
                .read_exact(&mut buf)
                .map(|_| buf[0] as i32)
                .unwrap_or(-1)
        },
        _ => unreachable!()
    }

}

/// Callback invoked during the execution of the Brainfuck program to intercept the output command `.`.
extern "C" fn putchar_interceptor(data: *const IoTarget, value: i32) -> i32 {
    
    // Write the byte to the output stream
    let data = unsafe { &*data };
    match data.output {
        OutputTarget::Custom(ref w) => {
            let buf = [ value as u8 ];
            w.borrow_mut()
                .write_all(&buf)
                .map(|_| value)
                .unwrap_or(-1)
        },
        _ => unreachable!()
    }

}

/// Compiled Brainfuck program, ready to be JITed or saved to disk.
pub struct CompiledProgram {
    module: Module,
    execution_engine: RefCell<Option<ExecutionEngine>>,
    optimization_level: OptimizationLevel,

    // The I/O streams must be kept alive if we are not using stdio
    io: Box<IoTarget>
}

impl CompiledProgram {

    /// Executes the compiled program.
    pub fn run(&self) {

        // This is the type of the main function we defined in `Compiler::new()`
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
    /// Panics if the program was compiled with custom I/O.
    pub fn save_object<P: AsRef<Path>>(&self, path: P) -> Result<(), BrainfuckError> {
        
        // Panic if we are using a custom stdio configuration
        if let InputTarget::Custom(_) = &self.io.input {
            panic!("Cannot save compiled program to disk when using custom I/O.");
        }
        if let OutputTarget::Custom(_) = &self.io.output {
            panic!("Cannot save compiled program to disk when using custom I/O.");
        }

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
    /// 
    /// Panics if the program was compiled with custom I/O.
    pub fn save_executable<P: AsRef<Path>>(&self, path: P) -> Result<(), BrainfuckError> {
        
        // Panic if we are using a custom stdio configuration
        if let InputTarget::Custom(_) = &self.io.input {
            panic!("Cannot save compiled program to disk when using custom I/O.");
        }
        if let OutputTarget::Custom(_) = &self.io.output {
            panic!("Cannot save compiled program to disk when using custom I/O.");
        }

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