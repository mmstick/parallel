use std::io;
use std::ffi::OsStr;
use std::process::{Child, Command, ExitStatus, Stdio};
use super::arguments::tokenizer::Token;
use super::arguments::token_matcher::*;
use super::arguments::InputIteratorErr;

pub enum CommandResult {
    Grouped(Child),
    Status
}

pub enum CommandErr {
    Input(InputIteratorErr),
    IO(io::Error)
}

pub struct ParallelCommand<'a> {
    pub slot_no:          &'a str,
    pub job_no:           &'a str,
    pub job_total:        &'a str,
    pub input:            &'a str,
    pub command_template: &'a [Token],
}

impl<'a> ParallelCommand<'a> {
    pub fn exec(&self, grouped: bool, uses_shell: bool, quiet: bool)
        -> Result<CommandResult, CommandErr>
    {
        // First the arguments will be generated based on the tokens and input.
        let mut arguments = String::with_capacity(self.command_template.len() << 1);
        try!(self.build_arguments(&mut arguments).map_err(CommandErr::Input));

        // Check to see if any placeholder tokens are in use.
        let placeholder_exists = self.command_template.iter().any(|x| {
            x == &Token::BaseAndExt || x == &Token::Basename || x == &Token::Dirname ||
            x == &Token::Job || x == &Token::Placeholder || x == &Token::RemoveExtension ||
            x == &Token::Slot
        });

        // If no placeholder tokens are in use, the user probably wants to infer one.
        if !placeholder_exists {
            arguments.push(' ');
            arguments.push_str(self.input);
        }

        if grouped {
            get_command_output(&arguments, uses_shell, quiet)
                .map(CommandResult::Grouped).map_err(CommandErr::IO)
        } else {
            get_command_status(&arguments, uses_shell, quiet)
                .map(|_| CommandResult::Status).map_err(CommandErr::IO)
        }
    }

    /// Builds arguments using the `tokens` template with the current `input` value.
    /// The arguments will be stored within a `Vec<String>`
    fn build_arguments(&self, arguments: &mut String) -> Result<(), InputIteratorErr> {
        for arg in self.command_template {
            match *arg {
                Token::Argument(ref arg)  => arguments.push_str(arg),
                Token::Basename           => arguments.push_str(basename(self.input)),
                Token::BaseAndExt         => arguments.push_str(basename(remove_extension(self.input))),
                Token::Dirname            => arguments.push_str(dirname(self.input)),
                Token::Job                => arguments.push_str(self.job_no),
                Token::Placeholder        => arguments.push_str(self.input),
                Token::RemoveExtension    => arguments.push_str(remove_extension(self.input)),
                Token::Slot               => arguments.push_str(self.slot_no)
            }
        }
        Ok(())
    }
}

pub fn get_command_output(command: &str, uses_shell: bool, quiet: bool) -> io::Result<Child> {
    if uses_shell {
        shell_output(command, quiet)
    } else {
        let arguments = split_into_args(command);
        match (arguments.len() == 1, quiet) {
            (true, true) => Command::new(&arguments[0])
                .stdout(Stdio::null()).stderr(Stdio::piped())
                .spawn(),
            (true, false) => Command::new(&arguments[0])
                .stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn(),
            (false, true) => Command::new(&arguments[0]).args(&arguments[1..])
                .stdout(Stdio::null()).stderr(Stdio::piped())
                .spawn(),
            (false, false) => Command::new(&arguments[0]).args(&arguments[1..])
                .stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn()
        }
    }
}

pub fn get_command_status(command: &str, uses_shell: bool, quiet: bool) -> io::Result<ExitStatus> {
    if uses_shell {
        shell_status(command, quiet)
    } else {
        let arguments = split_into_args(command);
        match (arguments.len() == 1, quiet) {
            (true, true)   => Command::new(&arguments[0]).stdout(Stdio::null()).status(),
            (true, false)  => Command::new(&arguments[0]).status(),
            (false, true)  => Command::new(&arguments[0]).args(&arguments[1..]).stdout(Stdio::null()).status(),
            (false, false) => Command::new(&arguments[0]).args(&arguments[1..]).status()
        }
    }
}

#[cfg(windows)]
fn shell_output<S: AsRef<OsStr>>(args: S, quiet: bool) -> io::Result<Child> {
    if quiet {
        Command::new("cmd").arg("/C").arg(args)
            .stdout(Stdio::null()).stderr(Stdio::piped())
            .spawn()
    } else {
        Command::new("cmd").arg("/C").arg(args)
            .stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn()
    }
}

#[cfg(windows)]
fn shell_status<S: AsRef<OsStr>>(args: S, quiet: bool) -> io::Result<ExitStatus> {
    if quiet {
        Command::new("cmd").arg("/C").arg(args).stdout(Stdio::null()).status()
    } else {
        Command::new("cmd").arg("/C").arg(args).status()
    }
}

#[cfg(not(windows))]
fn shell_output<S: AsRef<OsStr>>(args: S, quiet: bool) -> io::Result<Child> {
    if quiet {
        Command::new("sh").arg("-c").arg(args)
            .stdout(Stdio::null()).stderr(Stdio::piped())
            .spawn()
    } else {
        Command::new("sh").arg("-c").arg(args)
            .stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn()
    }
}

#[cfg(not(windows))]
fn shell_status<S: AsRef<OsStr>>(args: S, quiet: bool) -> io::Result<ExitStatus> {
    if quiet {
        Command::new("sh").arg("-c").arg(args).stdout(Stdio::null()).status()
    } else {
        Command::new("sh").arg("-c").arg(args).status()
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
