#[macro_use] extern crate log;

use std::fs::File;
use clap::{App, Arg, ArgMatches, SubCommand};
use itertools::Itertools;
use rustybf::{BrainfuckError, Instruction, Compiler, Interpreter, Optimizer};
use rustybf::parser::parse;
use rustybf::optimizer::ALL_OPTIMIZATIONS;

fn load_program(path: &str, optimizer: &Optimizer) -> Result<Vec<Instruction>, BrainfuckError> {
    
    // Parse the file
    debug!("Opening {}.", path);
    let file = File::open(path)?;
    debug!("Parsing source file.");
    let mut instructions = parse(file)?;
    info!("Source file {} loaded.", path);

    // Optimize the instructions
    instructions = optimizer.run(instructions);
    info!("Instructions optimized.");

    Ok(instructions)

}

fn run_list_optimizations() -> Result<(), BrainfuckError> {

    // Just print all the optimizations we have
    for name in ALL_OPTIMIZATIONS.keys() {
        println!("{}", name);
    }

    Ok(())

}

fn run_print_instructions(matches: &ArgMatches, optimizer: &Optimizer) -> Result<(), BrainfuckError> {

    // Load the program and print its instructions
    let instructions = load_program(matches.value_of("INPUT").unwrap(), optimizer)?;
    for i in &instructions {
        println!("{}", i);
    }

    Ok(())

}

fn run_exec(matches: &ArgMatches, optimizer: &Optimizer) -> Result<(), BrainfuckError> {
    
    let instructions = load_program(matches.value_of("INPUT").unwrap(), optimizer)?;

    // JIT is not implemented yet
    if matches.is_present("jit") {
        
        let optimization_level =
            matches.value_of("llvm-opt").unwrap()
            .parse::<u32>().map_err(|e| format!("Invalid value for llvm-opt: {}", e.to_string()))?;

        // Compile the program
        info!("Compiling program, optimization level {}.", optimization_level);
        let program =
            Compiler::new(optimization_level)
            .compile_instructions(&instructions)
            .finish();

        // Print the IR if we've been asked to do so
        if matches.is_present("print-llvm-ir") {
            program.dump(&mut std::io::stdout())?;
        }

        // Run the program
        info!("Executing program.");
        program.run();

    } else {

        info!("Executing program using interpreter.");

        // Prepare an interpreter to run the instructions
        let mut interpreter =
            Interpreter::builder()
            .input(std::io::stdin())
            .output(std::io::stdout())
            .build();

        // Aaaaand, run!
        interpreter.run(&instructions)?;

        // Print the whole tape in hex chars
        if matches.is_present("print-tape") {
            let tape = interpreter.tape().iter()
                .enumerate()
                .format_with(" ", |(i, x), f| {
                    if i == interpreter.tape_position() {
                        f(&format_args!("({:02X})", x))
                    } else {
                        f(&format_args!("{:02X}", x))
                    }
                });
            println!("[{}]", tape);
        }

    }

    Ok(())

}

fn run_compile(matches: &ArgMatches, optimizer: &Optimizer) -> Result<(), BrainfuckError> {
    
    let instructions = load_program(matches.value_of("INPUT").unwrap(), optimizer)?;

    let optimization_level =
        matches.value_of("llvm-opt").unwrap()
        .parse::<u32>().map_err(|e| format!("Invalid value for llvm-opt: {}", e.to_string()))?;

    // Compile the program
    info!("Compiling program, optimization level {}.", optimization_level);
    let program =
        Compiler::new(optimization_level)
        .compile_instructions(&instructions)
        .finish();

    // Print the IR if we've been asked to do so
    if matches.is_present("print-llvm-ir") {
        program.dump(&mut std::io::stdout())?;
    }

    // Save the program to disk
    let output = matches.value_of("output").unwrap();
    let obj = matches.is_present("obj");
    if obj {
        program.save_object(output)?;
        info!("Object file written at {}", output);
    } else {
        program.save_executable(output)?;
        info!("Executable written at {}", output);
    }

    Ok(())

}

fn run(matches: ArgMatches) -> Result<(), BrainfuckError> {
    
    // If we have been asked to just list the optimizations, do it and exit
    if matches.subcommand_matches("list-optimizations").is_some() {
        return run_list_optimizations();
    }

    // Prepare the optimizer
    let optimizer = Optimizer::with_passes_str(matches.value_of("optimizations").unwrap())?;
    if optimizer.passes().is_empty() {
        debug!("No optimizations selected.");
    } else {
        debug!("Selected optimization passes:");
        for pass in optimizer.passes() {
            debug!("  - {}", pass.name());
        }
    }

    // Decide what task to run depending on the subcommand used by the user
    match matches.subcommand() {
        ("print-instructions", Some(submatches)) => run_print_instructions(submatches, &optimizer),
        ("exec", Some(submatches)) => run_exec(submatches, &optimizer),
        ("compile", Some(submatches)) => run_compile(submatches, &optimizer),
        _ => {
            Err("Nothing to do.".into())
        }
    }

}

fn main() {

    // All the cli options are here
    let matches = App::new("rustybf")
        .version("0.1.0")
        .author("Marco Cameriero")
        .about("A Rusty Brainfuck compiler and interpreter")

        // Common options
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity. Repeat to increase.")
        )
        .arg(
            Arg::with_name("optimizations")
                .short("O")
                .long("optimizations")
                .takes_value(true)
                .default_value("all")
                .help("Specifies the optimizations to use")
        )

        // Subcommand: list-optimizations
        .subcommand(
            SubCommand::with_name("list-optimizations")
            .about("Lists all the possible optimizations implemented in rustybf")
        )

        // Subcommand: print-instructions
        .subcommand(
            SubCommand::with_name("print-instructions")
            .about("Prints the optimized instructions of a program and then exits")
            .arg(
                Arg::with_name("INPUT")
                    .help("Sets the input file to use")
                    .index(1)
                    .required(true)
            )
        )

        // Subcommand: exec
        .subcommand(
            SubCommand::with_name("exec")
            .about("Executes a Brainfuck program, either using the interpreter or the JIT")
            .arg(
                Arg::with_name("INPUT")
                    .help("Sets the input file to use")
                    .index(1)
                    .required(true)
            )
            .arg(
                Arg::with_name("print-tape")
                    .long("print-tape")
                    .conflicts_with("jit")
                    .help("Prints the value of the tape at the end of execution")
            )
            .arg(
                Arg::with_name("jit")
                    .short("j")
                    .long("jit")
                    .help("Use the JIT engine instead of the interpreter to execute the program")
            )
            .arg(
                Arg::with_name("llvm-opt")
                    .long("llvm-opt")
                    .help("Sets the LLVM optimization level for JIT compilation")
                    .requires("jit")
                    .takes_value(true)
                    .default_value_if("jit", None, "3")
            )
            .arg(
                Arg::with_name("print-llvm-ir")
                    .long("print-llvm-ir")
                    .help("Prints the LLVM IR generated for JIT compilation")
                    .requires("jit")
            )
        )

        // Subcommand: compile
        .subcommand(
            SubCommand::with_name("compile")
            .about("Compiles a Brainfuck program producing an executable file")
            .arg(
                Arg::with_name("INPUT")
                    .help("Sets the input file to use")
                    .index(1)
                    .required(true)
            )
            .arg(
                Arg::with_name("output")
                    .short("o")
                    .long("output")
                    .help("Path of the final file to create")
                    .required(true)
                    .takes_value(true)
            )
            .arg(
                Arg::with_name("obj")
                    .long("obj")
                    .help("Do not link the final executable. The output of the compilation will be an object file.")
            )
            .arg(
                Arg::with_name("llvm-opt")
                    .long("llvm-opt")
                    .help("Sets the LLVM optimization level for compilation")
                    .takes_value(true)
                    .default_value("3")
            )
            .arg(
                Arg::with_name("print-llvm-ir")
                    .long("print-llvm-ir")
                    .short("p")
                    .help("Prints to stdout the compiled LLVM IR")
            )
        )

        .get_matches();

    // Initialize logger as soon as possible
    let verbosity = match matches.occurrences_of("v") {
        0     => "warn",
        1     => "info",
        2     => "debug",
        3 | _ => "trace"
    };
    env_logger::Builder::from_env(
        env_logger::Env::new()
            .filter_or("RUSTYBF_LOG", format!("rustybf={}", verbosity))
            .write_style_or("RUSTYBF_LOG_STYLE", "auto")
    )
    .init();

    // Run the program
    if let Err(e) = run(matches) {
        error!("{}", e);
        std::process::exit(1);
    }

}
