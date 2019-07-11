use std::io::{Read, Write};
use crate::BrainfuckError;
use crate::parser::Instruction;

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
            tape: vec![0; self.tape_size],
            tape_position: 0,
            input: std::mem::replace(&mut self.input, None),
            output: std::mem::replace(&mut self.output, None)
        }
    }

}

pub struct Interpreter<R, W>
    where R: Read,
          W: Write
{
    tape: Vec<u8>,
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
    pub fn tape(&self) -> &[u8] {
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
                
                Instruction::Right { .. } => {
                    if self.tape_position == self.tape.len() - 1 {
                        return Err(BrainfuckError::TapeOverflow);
                    }
                    self.tape_position += 1;
                },
                
                Instruction::Left { .. } => {
                    if self.tape_position == 0 {
                        return Err(BrainfuckError::TapeUnderflow);
                    }
                    self.tape_position -= 1;
                },
                
                Instruction::Add { amount, .. } => {
                    let value = &mut self.tape[self.tape_position];
                    *value = value.wrapping_add(*amount);
                },

                Instruction::Sub { amount, .. } => {
                    let value = &mut self.tape[self.tape_position];
                    *value = value.wrapping_sub(*amount);
                },
                
                Instruction::Input { .. } => {
                    if let Some(ref mut input) = self.input {
                        input.read_exact(&mut self.tape[self.tape_position..=self.tape_position])
                            .map_err(BrainfuckError::IoError)?;
                    } else {
                        self.tape[self.tape_position] = 0;
                    }
                },
                
                Instruction::Output { .. } => {
                    if let Some(ref mut output) = self.output {
                        output.write_all(&self.tape[self.tape_position..=self.tape_position])
                            .map_err(BrainfuckError::IoError)?;
                    }
                },
                
                Instruction::Loop { ref body, .. } => {
                    while self.tape[self.tape_position] != 0 {
                        self.run(body)?;
                    }
                }

            }
        }

        Ok(())
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