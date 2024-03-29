use std::io::{Read, Write};
use std::num::Wrapping;
use crate::{BrainfuckError, Instruction};

/// Builder for the [`Interpreter`](crate::interpreter::Interpreter) struct.
pub struct InterpreterBuilder<R, W>
    where R: Read,
          W: Write
{
    tape_size: usize,
    input: Option<R>,
    output: Option<W>
}

impl<R, W> Default for InterpreterBuilder<R, W>
    where R: Read,
          W: Write
{
    fn default() -> Self {
        InterpreterBuilder::new()
    }
}

impl<R, W> InterpreterBuilder<R, W>
    where R: Read,
          W: Write
{

    /// Creates a new [`InterpreterBuilder`](crate::interpreter::InterpreterBuilder) with the default settings.
    pub fn new() -> InterpreterBuilder<R, W> {
        InterpreterBuilder {
            tape_size: 30_000,
            input: None,
            output: None
        }
    }

    /// Sets the maximum tape size.
    /// Panics if the size is set to zero.
    pub fn tape_size(&mut self, tape_size: usize) -> &mut Self {
        if tape_size == 0 {
            panic!("Tape size must be at least 1.");
        }
        self.tape_size = tape_size;
        self
    }

    /// Sets the stream that will be used as input for the `,` instruction.
    pub fn input(&mut self, input: R) -> &mut Self {
        self.input = Some(input);
        self
    }

    /// Sets the stream that will be used as output for the `.` instruction.
    pub fn output(&mut self, output: W) -> &mut Self {
        self.output = Some(output);
        self
    }

    /// Builds the actual [`Interpreter`](crate::interpreter::Interpreter).
    pub fn build(&mut self) -> Interpreter<R, W> {
        Interpreter {
            tape: vec![Wrapping(0); self.tape_size],
            tape_position: 0,
            input: std::mem::replace(&mut self.input, None),
            output: std::mem::replace(&mut self.output, None)
        }
    }

}

/// Main entrypoint of the Brainfuck interpreter.
/// This structure holds the state of the tape and can run a set of instructions.
pub struct Interpreter<R, W>
    where R: Read,
          W: Write
{
    tape: Vec<Wrapping<u8>>,
    tape_position: usize,
    input: Option<R>,
    output: Option<W>
}

impl<R, W> Default for Interpreter<R, W>
    where R: Read,
          W: Write
{
    fn default() -> Self {
        Interpreter::new()
    }
}

impl<R, W> Interpreter<R, W>
    where R: Read,
          W: Write
{

    /// Builds an [`Interpreter`](crate::interpreter::Interpreter) with the default settings.
    pub fn new() -> Interpreter<R, W> {
        InterpreterBuilder::new().build()
    }

    /// Creates an [`InterpreterBuilder`](crate::interpreter::InterpreterBuilder) to configure
    /// a new [`Interpreter`](crate::interpreter::Interpreter).
    pub fn builder() -> InterpreterBuilder<R, W> {
        InterpreterBuilder::new()
    }

    /// Returns a reference to the underlying tape used by this [`Interpreter`](crate::interpreter::Interpreter).
    pub fn tape(&self) -> &[Wrapping<u8>] {
        &*self.tape
    }

    /// Returns the position of the data pointer on the tape.
    pub fn tape_position(&self) -> usize {
        self.tape_position
    }

    /// Returns a reference to the input stream used by this [`Interpreter`](crate::interpreter::Interpreter).
    pub fn input(&self) -> Option<&R> {
        self.input.as_ref()
    }

    /// Returns a reference to the output stream used by this [`Interpreter`](crate::interpreter::Interpreter).
    pub fn output(&self) -> Option<&W> {
        self.output.as_ref()
    }

    /// Executes the given set of instructions in this [`Interpreter`](crate::interpreter::Interpreter).
    pub fn run(&mut self, instructions: &[Instruction]) -> Result<(), BrainfuckError> {
        for inst in instructions {
            match inst {
                
                Instruction::Move { offset, .. } => {
                    let new_offset = self.compute_offset(*offset)?;
                    self.tape_position = new_offset;
                },
                
                Instruction::Add { amount, .. } => {
                    let value = &mut self.tape[self.tape_position];
                    *value += *amount;
                },
                
                Instruction::Input { .. } => {
                    if let Some(ref mut input) = self.input {
                        let mut buf = [0u8];
                        input.read_exact(&mut buf).map_err(BrainfuckError::IoError)?;
                        self.tape[self.tape_position] = Wrapping(buf[0]);
                    } else {
                        self.tape[self.tape_position] = Wrapping(0);
                    }
                },
                
                Instruction::Output { .. } => {
                    if let Some(ref mut output) = self.output {
                        let buf = self.tape[self.tape_position].0;
                        output.write_all(&[buf]).map_err(BrainfuckError::IoError)?;
                        output.flush()?;
                    }
                },
                
                Instruction::Loop { ref body, .. } => {
                    while self.tape[self.tape_position] != Wrapping(0) {
                        self.run(body)?;
                    }
                },

                Instruction::Clear { .. } => {
                    self.tape[self.tape_position] = Wrapping(0);
                },

                Instruction::Mul { offset, amount, .. } => {
                    // To respect the proper loop semantics, if the current cell value is 0, do nothing.
                    // Multiplication is always a loop, thus is not executed if the current cell is 0.
                    // This is important because we might risk goind underflow/overflow for an operation
                    // which in reality is a noop.
                    if self.tape[self.tape_position] == Wrapping(0) {
                        continue;
                    }
                    let target_pos = self.compute_offset(*offset)?;
                    let tmp = self.tape[self.tape_position] * (*amount);
                    self.tape[target_pos] += tmp;
                }

            }
        }

        Ok(())
    }

    #[inline]
    fn compute_offset(&self, offset: isize) -> Result<usize, BrainfuckError> {
        let target_pos = (self.tape_position as isize) + offset;
        if target_pos < 0 {
            return Err(BrainfuckError::TapeUnderflow);
        }
        if target_pos >= self.tape.len() as isize {
            return Err(BrainfuckError::TapeOverflow);
        }
        Ok(target_pos as usize)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::parser::parse;

    fn assert_prog(prog: &str, input: &str, expected_output: &str) {
        let i: Cursor<&[u8]> = Cursor::new(input.as_bytes());
        let o: Cursor<Vec<u8>> = Cursor::new(Vec::new());

        let mut interpreter = Interpreter::builder()
            .input(i)
            .output(o)
            .build();

        interpreter.run(&parse(Cursor::new(prog)).unwrap()).unwrap();

        let actual_output = interpreter.output().unwrap().get_ref();
        assert_eq!(actual_output.as_slice(), expected_output.as_bytes());
    }

    #[test]
    fn test_simple1() {
        // Taken from: https://en.wikipedia.org/wiki/Brainfuck
        let prog = r#"
            ++       Cell c0 = 2
            > +++++  Cell c1 = 5
            
            [            Start your loops with your cell pointer on the loop counter (c1 in our case)
                < +      Add 1 to c0
                > -      Subtract 1 from c1
            ]            End your loops with the cell pointer on the loop counter
            
            At this point our program has added 5 to 2 leaving 7 in c0 and 0 in c1
            but we cannot output this value to the terminal since it is not ASCII encoded!
            
            To display the ASCII character "7" we must add 48 to the value 7
            48 = 6 * 8 so let's use another loop to help us!
            
            ++++ ++++      c1 = 8 and this will be our loop counter again
            [
                < +++ +++  Add 6 to c0
                > -        Subtract 1 from c1
            ]
            < .            Print out c0 which has the value 55 which translates to "7"!
        "#;

        assert_prog(prog, "", "7");
    }

    #[test]
    fn test_simple2() {
        // Taken from: https://en.wikipedia.org/wiki/Brainfuck
        let prog = r#"
            ++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.
        "#;

        assert_prog(prog, "", "Hello World!\n");
    }

    #[test]
    fn test_input() {
        let prog = ",+.,+.";
        assert_prog(prog, "AB", "BC");
    }

    #[test]
    fn test_underflow() {
        let prog = Cursor::new("<");
        assert!(Interpreter::<Cursor<&[u8]>, Cursor<Vec<u8>>>::new().run(&parse(prog).unwrap()).is_err());
    }

    #[test]
    fn test_overflow() {
        let prog = Cursor::new(">>");
        assert!(
            Interpreter::<Cursor<&[u8]>, Cursor<Vec<u8>>>::builder()
            .tape_size(2)
            .build()
            .run(&parse(prog).unwrap())
            .is_err()
        );
    }
}