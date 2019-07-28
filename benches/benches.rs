#[macro_use]
extern crate criterion;
#[macro_use]
extern crate lazy_static;

use std::io::{Cursor, Write};
use std::fmt;
use std::process::{Command, Stdio};
use criterion::{Criterion, ParameterizedBenchmark};
use tempfile::NamedTempFile;
use rustybf::{Instruction, Optimizer, Compiler, Interpreter};
use rustybf::parser::parse;

struct Program<'a> {
    name: &'a str,
    raw_program: &'a [u8],
    input: &'a [u8],
    optimized_instructions: Vec<Instruction>
}

impl<'a> fmt::Debug for Program<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

macro_rules! program {
    ($name:ident) => {
        {
            let raw_program: &[u8] = include_bytes!(concat!("../tests/programs/", stringify!($name), ".b"));
            let instr = parse(Cursor::new(raw_program)).unwrap();
            let optimized_instructions = Optimizer::with_passes_str("all").unwrap().run(instr);
            Program {
                name: stringify!($name),
                raw_program,
                input: include_bytes!(concat!("../tests/programs/", stringify!($name), ".b.in")),
                optimized_instructions
            }
        }
    };
}

lazy_static! {
    static ref PROGRAMS: [Program<'static>; 5] = [
        program!(hello_world),
        program!(factor),
        program!(hanoi),
        program!(mandelbrot),
        program!(dbfi)
    ];
}

// Benchmark for the parser
fn parser_benches(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "Parser",
        move |b, &program| {
            b.iter(|| parse(Cursor::new(program.raw_program)).unwrap());
        },
        &*PROGRAMS
    );

}

// Comparison of execution of the same programs with both interpreter and jit
fn interpreted_vs_compiled(c: &mut Criterion) {

    fn run_interpreter(p: &Program<'static>) {
        let mut interpreter =
            Interpreter::builder()
            .input(Cursor::new(p.input))
            .output(Cursor::new(Vec::new()))
            .build();
        interpreter.run(&p.optimized_instructions).unwrap();
    }

    fn run_compiled(p: &Program<'static>) {
        let program =
            Compiler::new(3)
            .compile_instructions(&p.optimized_instructions)
            .finish();
        let path = NamedTempFile::new().unwrap().into_temp_path();
        program.save_executable(&path).unwrap();

        let mut child = Command::new(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.as_mut().unwrap().write_all(p.input).unwrap();
        let status = child.wait().unwrap();

        if !status.success() {
            panic!("Compiled program failed");
        }
    }

    // For each program, bench the performance of the interpreter and of the jit
    c.bench("Execution",
        ParameterizedBenchmark::new(
            "Interpreter",
            |b, p| b.iter(|| run_interpreter(p)),
            &*PROGRAMS
        )
        .with_function(
            "Compiled",
            |b, p| b.iter(|| run_compiled(p))
        )
    );

}

criterion_group!(benches, parser_benches, interpreted_vs_compiled);
criterion_main!(benches);