use std::env;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::process::{Child, Command, Stdio};
use super::tokenizer::*;
use super::arguments;

pub enum CommandErr {
    IO(io::Error)
}

/// If no placeholder tokens are in use, then the input will be appended at the end of the the command.
fn append_argument(arguments: &mut String, command_template: &[Token], input: &str) {
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

pub struct ParallelCommand<'a> {
    pub slot_no:          &'a str,
    pub job_no:           &'a str,
    pub job_total:        &'a str,
    pub input:            &'a str,
    pub command_template: &'a [Token],
}

impl<'a> ParallelCommand<'a> {
    pub fn exec(&self, flags: u8) -> Result<Child, CommandErr> {
        // First the arguments will be generated based on the tokens and input.
        let mut arguments = String::with_capacity(self.command_template.len() << 1);
        self.build_arguments(&mut arguments, flags & arguments::PIPE_IS_ENABLED != 0);

        if flags & arguments::PIPE_IS_ENABLED == 0 {
            append_argument(&mut arguments, self.command_template, self.input);
            get_command_output(&arguments, flags).map_err(CommandErr::IO)
        } else {
            let mut child = get_command_output(&arguments, flags).map_err(CommandErr::IO)?;

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
    fn build_arguments(&self, arguments: &mut String, pipe: bool) {
        if pipe {
            for arg in self.command_template {
                match *arg {
                    Token::Argument(ref arg) => arguments.push_str(arg),
                    Token::Job               => arguments.push_str(self.job_no),
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
                    Token::Job               => arguments.push_str(self.job_no),
                    Token::Placeholder       => arguments.push_str(self.input),
                    Token::RemoveExtension   => arguments.push_str(remove_extension(self.input)),
                    Token::Slot              => arguments.push_str(self.slot_no)
                }
            }
        }
    }
}

pub fn get_command_output(command: &str, flags: u8) -> io::Result<Child> {
    if flags & arguments::SHELL_ENABLED != 0 && flags & arguments::PIPE_IS_ENABLED == 0 {
        shell_output(command, flags)
    } else {
        let arguments = split_into_args(command);
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

fn shell_output<S: AsRef<OsStr>>(args: S, flags: u8) -> io::Result<Child> {
    let (cmd, flag) = if cfg!(windows) {
        ("cmd".to_owned(), "/C")
    } else {
        (env::var("SHELL").unwrap_or("sh".to_owned()), "-c")
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

/// Handles quoting of arguments to prevent arguments with spaces as being read as
/// multiple separate arguments. This is only executed when `--no-shell` is in use.
fn split_into_args(command: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut buffer = String::new();
    let mut quoted = false;
    let mut prev_char_was_a_backslash = false;
    for character in command.chars() {
        if quoted {
            match character {
                '\\' => {
                    if prev_char_was_a_backslash {
                        buffer.push('\\');
                        prev_char_was_a_backslash = false;
                    } else {
                        prev_char_was_a_backslash = true;
                    }
                },
                '"' => {
                    if prev_char_was_a_backslash {
                        buffer.push('\\');
                        prev_char_was_a_backslash = false;
                    } else {
                        if !buffer.is_empty() {
                            output.push(buffer.clone());
                            buffer.clear();
                        }
                        quoted = false;
                    }
                },
                _ => {
                    if prev_char_was_a_backslash {
                        buffer.push('\\');
                        prev_char_was_a_backslash = false;
                    }
                    buffer.push(character);
                }
            }
        } else {
            match character {
                ' ' => {
                    if prev_char_was_a_backslash {
                        buffer.push(' ');
                        prev_char_was_a_backslash = false;
                    } else if !buffer.is_empty() {
                        output.push(buffer.clone());
                        buffer.clear();
                    }
                },
                '\\' => {
                    if prev_char_was_a_backslash {
                        buffer.push('\\');
                        prev_char_was_a_backslash = false;
                    } else {
                        prev_char_was_a_backslash = true;
                    }
                },
                '"' => {
                    if prev_char_was_a_backslash {
                        buffer.push('"');
                        prev_char_was_a_backslash = false;
                    } else {
                        quoted = true;
                    }
                },
                _ => {
                    if prev_char_was_a_backslash {
                        buffer.push('\\');
                        prev_char_was_a_backslash = false;
                    } else {
                        buffer.push(character);
                    }
                }
            }
        }
    }

    if !buffer.is_empty() { output.push(buffer); }
    output.shrink_to_fit();
    output
}

#[test]
fn test_split_args() {
    let argument = "ffmpeg -i \"file with spaces\" \"output with spaces\"";
    let expected = vec!["ffmpeg", "-i", "file with spaces", "output with spaces"];
    assert_eq!(split_into_args(argument), expected);

    let argument = "one\\ two\\\\ three";
    let expected = vec!["one two\\", "three"];
    assert_eq!(split_into_args(argument), expected);
}
