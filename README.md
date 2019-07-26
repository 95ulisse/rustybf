# RustyBF

An optimizing compiler, interpreter and JIT for Brainfuck.

API docs: [https://95ulisse.github.io/rustybf](https://95ulisse.github.io/rustybf)

## Install `rustybf`

To install the binary `rustybf`, clone the repo and use `cargo` to install it:

```
git clone https://github.com/95ulisse/rustybf.git
cd rustybf

# Use `cargo install` to globally install `rustybf`
cargo install --path .

# Or use `cargo run` if you just want to run it
cargo run
```

## Examples

Execute a program using the **interpreter**:

```
$ rustybf exec hello_word.b
hello world
```

Execute a program by compiling it to **native code in memory** than run it:

```
$ rustybf exec --jit hello_word.b
hello world
```

Compile a program to an **executable file** and run it:

```
$ rustybf compile hello_world.b -o hello_world
$ file hello_world
hello_world: ELF 64-bit LSB pie executable, x86-64, ...
$ ./hello_world
hello world
```

## API

`rustybf` can also be used as a library to integrate Brainfuck within your own program (I mean, who wouldn't?).

```rust
use rustybf::{Compiler, Interpreter, Optimizer};
use rustybf::parser::parse;

// Parse the source file
let file = File::open("hello_world.b").unwrap();
let mut instructions = parse(file).unwrap();

// Optimize the instructions
// (use `rustybf::optimizer::DEFAULT_OPTIMIZATION_PASSES` for the default passes)
let optimizer = Optimizer::with_passes_str("collapse-increments,mul-loops,dead-code").unwrap();
instructions = optimizer.run(instructions);

// Now we can ether prepare an interpreter to run the instructions, or...
let mut interpreter =
    Interpreter::builder()
    .input(std::io::stdin())
    .output(std::io::stdout())
    .build();
interpreter.run(&instructions).unwrap();

// ... JIT compile the program and jump right to it
let program =
    Compiler::new(3) // 3 is the LLVM optimization level
    .compile_instructions(&instructions)
    .finish();
program.run();
```

## Optimizations

Here's a list of all the optimizations implemented in `rustybf`. To select the list of optimizations to apply use the `-O` option.

Note: `print-instructions` is a debug command that prints the instructions of a program after optimization and then exits.

### `collapse-increments`

Brainfuck programs always have long sequences of repeated `+`, `-`, `<` and `>`.
Instead of executing multiple increments/decrements on the same cell (the same applies to movements of the pointer),
precompute the amount to increment at compile time.

```
$ rustybf -O none print-instructions <(echo "+++")
Add(1)
Add(1)
Add(1)
$ rustybf -O collapse-increments print-instructions <(echo "+++")
Add(3)
```

### `clear-loops`

Another common idiom in Brainfuck programs is `[-]`, which is a loop that decrements the value of the current cell
until it reaches zero. Instead of wasting an amount of steps doing decrements of 1, directly set the value of the current cell to zero.

```
$ rustybf -O none print-instructions <(echo "[-]")
Loop {
    Add(255) // Note: cells are unsigned bytes. Adding 255 equals to subtracting 1.
}
$ rustybf -O clear-loops print-instructions <(echo "[-]")
Clear
```

### `mul-loops`

Brainfuck provides only increment and decrement primitives, there's no multiplication neither division:
these kind of arithmetic operations must be explicitely expressed as loops.

For example, `[->++<]` sets the value of the cell to the right of the current one to twice the value of the current cell.

The pattern can be extended to more complex constructs, like `[->++>+++<<<->]`,
which modified the value of the two cells to the right and of the cell to the left.

```
$ rustybf -O none print-instructions <(echo "[->++>+++<<<->]")
Loop {
    Add(255)
    Move <+1>
    Add(1)
    Add(1)
    Move <+1>
    Add(1)
    Add(1)
    Add(1)
    Move <-1>
    Move <-1>
    Move <-1>
    Add(255)
    Move <+1>
}
$ rustybf -O mul-loops print-instructions <(echo "[->++>+++<<<->]")
Mul(2) <+1>
Mul(255) <-1>
Mul(3) <+2>
Clear
```

A note on the syntax: `Mul(x) <y>` means *take the value of the current cell, multiply it by `x`
and add it to the cell at offset `y` from the current one*.

### `dead-code`

Dead simple dead code removal:

- Remove null increments or pointer movements (`Add(0)`, `Move <0>`).
- Loops at the beginning of the program are never executed since the cells are all initialized
  to `0`, remove them directly.
- Consecutive loops are never executed: in code like `[A][B]`, if we ever exit from loop `[A]`,
  it means that the current cell value is `0`, so loop `[B]` will never be executed.
  We can safely remove consecutive loops and just keep the first.

```
$ rustybf -O dead-code print-instructions <(echo "[+]+")
Add(1)
$ rustybf -O dead-code print-instructions <(echo "+[+][-]")
Add(1)
Loop {
    Add(1)
}
```

## License

`rustybf` is released under the MIT license. For more information, see [LICENSE](LICENSE).