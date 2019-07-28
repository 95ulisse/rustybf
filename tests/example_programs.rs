use std::io::{Cursor, Write};
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;
use rustybf::{BrainfuckError, Interpreter, Compiler, Optimizer};
use rustybf::parser::parse;

fn run(program: &[u8], input: &[u8], expected: &[u8]) -> Result<(), BrainfuckError> {
    
    // Parse the file
    let mut instructions = parse(Cursor::new(program))?;

    // Optimize the instructions
    instructions = Optimizer::with_passes_str("all")?.run(instructions);

    // Prepare an interpreter to run the instructions
    let mut interpreter =
        Interpreter::builder()
        .input(Cursor::new(input))
        .output(Cursor::new(Vec::new()))
        .build();

    // Aaaaand, run!
    interpreter.run(&instructions)?;

    // Check that the output of the interpreter matches the expected one
    if interpreter.output().unwrap().get_ref().as_slice() != expected {
        return Err("Mismatching output".into());
    }

    Ok(())

}

fn run_compiled(program: &[u8], input: &[u8], expected: &[u8]) -> Result<(), BrainfuckError> {
    
    // Parse the file
    let mut instructions = parse(Cursor::new(program))?;

    // Optimize the instructions
    instructions = Optimizer::with_passes_str("all")?.run(instructions);

    // Compile the instructions to a temporary file
    let program =
        Compiler::new(3)
        .compile_instructions(&instructions)
        .finish();
    let path = NamedTempFile::new()?.into_temp_path();
    program.save_executable(&path)?;    

    // Run the compiled program
    let mut child = Command::new(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(input)?;
    let output = child.wait_with_output().unwrap();

    // Check that the output of the interpreter matches the expected one
    if !output.status.success() {
        return Err(format!("Child process exited with status: {}", output.status).into());
    }
    if output.stdout.as_slice() != expected {
        return Err("Mismatching output".into());
    }

    Ok(())

}

// A test for each program

macro_rules! test_program {
    ($name:ident) => {
        paste::item! {
            #[test]
            fn [<test_ $name>]() {
                let program = include_bytes!(concat!("./programs/", stringify!($name), ".b"));
                let input = include_bytes!(concat!("./programs/", stringify!($name), ".b.in"));
                let output = include_bytes!(concat!("./programs/", stringify!($name), ".b.out"));
                run(program, input, output).unwrap();
            }

            #[test]
            fn [<test_ $name _compiled>]() {
                let program = include_bytes!(concat!("./programs/", stringify!($name), ".b"));
                let input = include_bytes!(concat!("./programs/", stringify!($name), ".b.in"));
                let output = include_bytes!(concat!("./programs/", stringify!($name), ".b.out"));
                run_compiled(program, input, output).unwrap();
            }
        }
    };
}

test_program!(hello_world);
test_program!(factor);
test_program!(hanoi);
test_program!(mandelbrot);
test_program!(dbfi);