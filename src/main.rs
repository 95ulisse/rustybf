#[macro_use] extern crate log;

use std::fs::File;
use clap::{App, Arg};
use rustybf::BrainfuckError;
use rustybf::parser::parse;
use rustybf::interpreter::Interpreter;

fn main_execute(path: &str) -> Result<(), BrainfuckError> {
    
    // Parse the file
    debug!("Opening {}.", path);
    let file = File::open(path)?;
    debug!("Parsing source file.");
    let instructions = parse(file)?;

    // Prepare an interpreter to run the instructions
    let mut interpreter =
        Interpreter::builder()
        .input(std::io::stdin())
        .output(std::io::stdout())
        .build();

    // Aaaaand, run!
    debug!("Running program.");
    interpreter.run(&instructions)?;
    debug!("Done.");

    Ok(())

}

fn main_compile(_path: &str) -> Result<(), BrainfuckError> {
    Err("Compile mode is not implemented yet.".into())
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
                .required(true)
                .index(1)
        )
        .arg(
            Arg::with_name("execute")
                .short("e")
                .long("execute")
                .help("Executes the given Brainfuck file without compiling it")
        )
        .arg(
            Arg::with_name("compile")
                .short("c")
                .long("compile")
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
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity. Repeat to increase.")
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

    // Check if we are in compile or execute mode
    let file = matches.value_of("INPUT").unwrap();
    let do_execute = matches.is_present("execute");
    let do_compile = matches.is_present("compile");
    let res = match (do_execute, do_compile) {
        (true, true) => {
            Err("Both switches for compile and execute mode cannot be present at the same time.".into())
        },
        (false, true) => {
            main_compile(file)
        },
        (true, false) => {
            main_execute(file)
        }
        (false, false) => {
            // Default to compile mode
            main_compile(file)
        }
    };

    if let Err(e) = res {
        error!("{}", e);
        std::process::exit(1);
    }
}
