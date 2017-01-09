use std::convert::AsRef;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::process::{Child, Command, Stdio};
use arguments;
use tokenizer::*;

pub enum CommandErr {
    IO(io::Error)
}

/// If no placeholder tokens are in use, then the input will be appended at the end of the the command.
pub fn append_argument(arguments: &mut String, command_template: &[Token], input: &str) {
    // Check to see if any placeholder tokens are in use.
    let placeholder_exists = command_template.iter().any(|x| {
        x == &Token::BaseAndExt || x == &Token::Basename || x == &Token::Dirname ||
        x == &Token::Job || x == &Token::Placeholder || x == &Token::RemoveExtension ||
        x == &Token::Slot
    });

    // If no placeholder tokens are in use, the user probably wants to infer one.
    if !placeholder_exists {
        arguments.push(' ');
        arguments.push_str(input);
    }
}

/// A structure for generating commands to be executed.
pub struct ParallelCommand<'a> {
    pub slot_no:          &'a str,
    pub job_no:           &'a [u8],
    pub job_total:        &'a [u8],
    pub input:            &'a str,
    pub flags:            u16,
    pub command_template: &'a [Token],
}

impl<'a> ParallelCommand<'a> {
    /// Builds and execute commands based on given flags, supplied inputs and token arguments.
    pub fn exec(&self, arguments: &mut String) -> Result<Child, CommandErr> {
        self.build_arguments(arguments);

        if self.flags & arguments::PIPE_IS_ENABLED == 0 {
            append_argument(arguments, self.command_template, self.input);
            get_command_output(arguments.as_str(), self.flags).map_err(CommandErr::IO)
        } else {
            let mut child = get_command_output(arguments.as_str(), self.flags).map_err(CommandErr::IO)?;

            {   // Grab a handle to the child's stdin and write the input argument to the child's stdin.
                let stdin = child.stdin.as_mut().unwrap();
                stdin.write(self.input.as_bytes()).map_err(CommandErr::IO)?;
                stdin.write(b"\n").map_err(CommandErr::IO)?;
            }

            // Drop the stdin of the child process to avoid having the application hang waiting for user input.
            drop(child.stdin.take());

            Ok(child)
        }
    }

    /// Builds arguments using the `tokens` template with the current `input` value.
    /// The arguments will be stored within a `Vec<String>`
    pub fn build_arguments(&self, arguments: &mut String) {
        if self.flags & arguments::PIPE_IS_ENABLED != 0 {
            for arg in self.command_template {
                match *arg {
                    Token::Argument(ref arg) => arguments.push_str(arg),
                    Token::Job               => for character in self.job_no { arguments.push(*character as char); },
                    Token::Slot              => arguments.push_str(self.slot_no),
                    _ => ()
                }
            }
        } else {
            for arg in self.command_template {
                match *arg {
                    Token::Argument(ref arg) => arguments.push_str(arg),
                    Token::Basename          => arguments.push_str(basename(self.input)),
                    Token::BaseAndExt        => arguments.push_str(basename(remove_extension(self.input))),
                    Token::Dirname           => arguments.push_str(dirname(self.input)),
                    Token::Job               => for character in self.job_no { arguments.push(*character as char); },
                    Token::Placeholder       => arguments.push_str(self.input),
                    Token::RemoveExtension   => arguments.push_str(remove_extension(self.input)),
                    Token::Slot              => arguments.push_str(self.slot_no)
                }
            }
        }
    }
}

/// Handles shell execution and returns a handle to the underlying `Child` process.
/// If the command requires to be executed in a shell, it will be executed within a shell.
/// Otherwise, the arguments will be split and the command will run without a shell.
pub fn get_command_output(command: &str, flags: u16) -> io::Result<Child> {
    if flags & arguments::SHELL_ENABLED != 0 && flags & arguments::PIPE_IS_ENABLED == 0 {
        shell_output(command, flags)
    } else {
        let arguments = ArgumentSplitter::new(command).collect::<Vec<String>>();
        match (arguments.len() == 1, flags & arguments::QUIET_MODE != 0, flags & arguments::PIPE_IS_ENABLED != 0) {
            (true, true, false) => Command::new(&arguments[0])
                .stdout(Stdio::null()).stderr(Stdio::piped())
                .spawn(),
            (true, true, true) => Command::new(&arguments[0])
                .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped())
                .spawn(),
            (true, false, false) => Command::new(&arguments[0])
                .stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn(),
            (true, false, true) => Command::new(&arguments[0])
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn(),
            (false, true, false) => Command::new(&arguments[0]).args(&arguments[1..])
                .stdout(Stdio::null()).stderr(Stdio::piped())
                .spawn(),
            (false, true, true) => Command::new(&arguments[0]).args(&arguments[1..])
                .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped())
                .spawn(),
            (false, false, false) => Command::new(&arguments[0]).args(&arguments[1..])
                .stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn(),
            (false, false, true) => Command::new(&arguments[0]).args(&arguments[1..])
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn(),
        }
    }
}

/// Executes the command within a shell
fn shell_output<S: AsRef<OsStr>>(args: S, flags: u16) -> io::Result<Child> {
    let (cmd, flag) = if cfg!(windows) {
        ("cmd".to_owned(), "/C")
    } else if flags & arguments::DASH_EXISTS != 0  {
        ("dash".to_owned(), "-c")
    } else {
        ("sh".to_owned(), "-c")
    };

    match (flags & arguments::QUIET_MODE != 0, flags & arguments::PIPE_IS_ENABLED != 0) {
        (true, false) => Command::new(cmd).arg(flag).arg(args)
            .stdout(Stdio::null()).stderr(Stdio::piped())
            .spawn(),
        (true, true) => Command::new(cmd).arg(flag).arg(args)
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped())
            .spawn(),
        (false, false) => Command::new(cmd).arg(flag).arg(args)
            .stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn(),
        (false, true) => Command::new(cmd).arg(flag).arg(args)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn()
    }
}

const DOUBLE: u8 = 1;
const SINGLE: u8 = 2;
const BACK:   u8 = 4;

/// An efficient `Iterator` structure for splitting arguments
struct ArgumentSplitter<'a> {
    buffer:       String,
    data:         &'a str,
    read:         usize,
    flags:        u8,
}

impl<'a> ArgumentSplitter<'a> {
    fn new(data: &'a str) -> ArgumentSplitter<'a> {
        ArgumentSplitter {
            buffer:       String::with_capacity(32),
            data:         data,
            read:         0,
            flags:        0,
        }
    }
}

impl<'a> Iterator for ArgumentSplitter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        for character in self.data.chars().skip(self.read) {
            self.read += 1;
            match character {
                _ if self.flags & BACK != 0 => {
                    self.buffer.push(character);
                    self.flags ^= BACK;
                },
                '"'  if self.flags & SINGLE == 0 => self.flags ^= DOUBLE,
                '\'' if self.flags & DOUBLE == 0 => self.flags ^= SINGLE,
                ' '  if !self.buffer.is_empty() & (self.flags & (SINGLE + DOUBLE) == 0) => break,
                '\\' if (self.flags & (SINGLE + DOUBLE) == 0) => self.flags ^= BACK,
                _ => self.buffer.push(character)
            }
        }

        if self.buffer.is_empty() {
            None
        } else {
            let mut output = self.buffer.clone();
            output.shrink_to_fit();
            self.buffer.clear();
            Some(output)
        }
    }
}

#[test]
fn test_split_args() {
    let argument = ArgumentSplitter::new("ffmpeg -i \"file with spaces\" \"output with spaces\"");
    let expected = vec!["ffmpeg", "-i", "file with spaces", "output with spaces"];
    assert_eq!(argument.collect::<Vec<String>>(), expected);

    let argument = ArgumentSplitter::new("one\\ two\\\\ three");
    let expected = vec!["one two\\", "three"];
    assert_eq!(argument.collect::<Vec<String>>(), expected);
}
