#[macro_use] extern crate log;

use std::fs::File;
use clap::{App, Arg, ArgMatches};
use rustybf::BrainfuckError;
use rustybf::parser::{parse, Instruction};
use rustybf::interpreter::Interpreter;
use rustybf::optimizer::{Optimizer, ALL_OPTIMIZATIONS};

fn run_print_instructions(instructions: Vec<Instruction>) -> Result<(), BrainfuckError> {
    for i in &instructions {
        println!("{}", i);
    }
    Ok(())
}

fn run_execute(instructions: Vec<Instruction>) -> Result<(), BrainfuckError> {
    
    info!("Executing program...");

    // Prepare an interpreter to run the instructions
    let mut interpreter =
        Interpreter::builder()
        .input(std::io::stdin())
        .output(std::io::stdout())
        .build();

    // Aaaaand, run!
    interpreter.run(&instructions)?;

    Ok(())

}

fn run_compile(_instructions: Vec<Instruction>) -> Result<(), BrainfuckError> {
    Err("Compile mode is not implemented yet.".into())
}

fn run(matches: ArgMatches) -> Result<(), BrainfuckError> {
    
    // If we have been asked to just list the optimizations, do it and exit
    if matches.is_present("list-optimizations") {
        for name in ALL_OPTIMIZATIONS.keys() {
            println!("{}", name);
        }
        return Ok(());
    }

    // Parse the file
    let path = matches.value_of("INPUT").unwrap();
    debug!("Opening {}...", path);
    let file = File::open(path)?;
    debug!("Parsing source file...");
    let mut instructions = parse(file)?;
    info!("Source file {} loaded.", path);

    // Prepare and run the optimizer
    let optimizer = Optimizer::with_passes_str(matches.value_of("optimizations").unwrap())?;
    if optimizer.passes().is_empty() {
        debug!("No optimizations selected.");
    } else {
        debug!("Selected optimization passes:");
        for pass in optimizer.passes() {
            debug!("  - {}", pass.name());
        }

        instructions = optimizer.run(instructions);
        info!("Instructions optimized.");
    }

    // Check what we have to do now
    let do_print = matches.is_present("print-instructions");
    let do_execute = matches.is_present("execute");
    let do_compile = matches.is_present("compile");
    match (do_print, do_execute, do_compile) {
        (true,  _,     _    ) => run_print_instructions(instructions),
        (false, false, false) => run_compile(instructions),
        (false, false, true ) => run_compile(instructions),
        (false, true,  false) => run_execute(instructions),
        (false, true,  true ) => unreachable!()
    }

}

fn main() {

    // All the cli options are here
    let matches = App::new("rustybf")
        .version("0.1.0")
        .author("Marco Cameriero")
        .about("A Rusty Brainfuck compiler")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required_unless("list-optimizations")
                .index(1)
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity. Repeat to increase.")
        )
        .arg(
            Arg::with_name("execute")
                .short("e")
                .long("execute")
                .conflicts_with("compile")
                .help("Executes the given Brainfuck file without compiling it")
        )
        .arg(
            Arg::with_name("compile")
                .short("c")
                .long("compile")
                .conflicts_with("execute")
                .help("Compiles the given Brainfuck file producing an executable")
        )
        .arg(
            Arg::with_name("optimizations")
                .short("O")
                .long("optimizations")
                .takes_value(true)
                .default_value("all")
                .help("Specifies the optimizations to use")
        )
        .arg(
            Arg::with_name("list-optimizations")
                .long("list-optimizations")
                .help("Prints a list of all available optimizations and exits")
        )
        .arg(
            Arg::with_name("print-instructions")
                .long("print-instructions")
                .help("Prints the optimized instructions and exits")
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
