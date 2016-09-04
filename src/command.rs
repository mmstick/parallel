use std::io;
use std::ffi::OsStr;
use std::process::{Child, Command, ExitStatus, Stdio};
use tokenizer::Token;

pub enum CommandResult {
    Grouped,
    Status
}

pub struct ParallelCommand<'a> {
    pub slot_no:          &'a str,
    pub job_no:           &'a str,
    pub job_total:        &'a str,
    pub input:            &'a str,
    pub command_template: &'a [Token],
}

impl<'a> ParallelCommand<'a> {
    pub fn exec(&self, grouped: bool, uses_shell: bool, child: &mut Child, jobs: &[String])
        -> io::Result<CommandResult>
    {
        // First the arguments will be generated based on the tokens and input.
        let mut arguments = String::with_capacity(self.command_template.len() << 1);
        self.build_arguments(&mut arguments, jobs);

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
            get_command_output(&arguments, uses_shell, child).map(|_| CommandResult::Grouped)
        } else {
            get_command_status(&arguments, uses_shell).map(|_| CommandResult::Status)
        }
    }

    /// Builds arguments using the `tokens` template with the current `input` value.
    /// The arguments will be stored within a `Vec<String>`
    fn build_arguments(&self, arguments: &mut String, jobs: &[String]) {
        for arg in self.command_template {
            match arg.clone() {
                Token::Argument(ref arg)  => arguments.push_str(arg),
                Token::Basename           => arguments.push_str(basename(self.input)),
                Token::BaseAndExt         => arguments.push_str(basename(remove_extension(self.input))),
                Token::Dirname            => arguments.push_str(dirname(self.input)),
                Token::Job                => arguments.push_str(self.job_no),
                Token::JobTotal           => arguments.push_str(self.job_total),
                Token::Number(job, token) => {
                    // The `token` is a pointer which needs to be unboxed.
                    let raw_token = Box::into_raw(token);
                    let input = &jobs[job-1];

                    unsafe {
                        // Match the associated token to be used with the job number.
                        // Unreachable patterns are not possible combinations by the tokenizer.
                        match *raw_token {
                            Token::Argument(_)     => unreachable!(),
                            Token::Basename        => arguments.push_str(basename(input)),
                            Token::BaseAndExt      => arguments.push_str(basename(remove_extension(input))),
                            Token::Dirname         => arguments.push_str(dirname(input)),
                            Token::Job             => unreachable!(),
                            Token::JobTotal        => unreachable!(),
                            Token::Number(_, _)    => unreachable!(),
                            Token::Placeholder     => arguments.push_str(input),
                            Token::RemoveExtension => arguments.push_str(remove_extension(input)),
                            Token::Slot            => unreachable!()
                        }
                    }
                }
                Token::Placeholder     => arguments.push_str(self.input),
                Token::RemoveExtension => arguments.push_str(remove_extension(self.input)),
                Token::Slot            => arguments.push_str(self.slot_no)
            }
        }
    }
}

pub fn get_command_output(command: &str, uses_shell: bool, child: &mut Child)
    -> io::Result<()>
{
    if uses_shell {
        shell_output(command, child)
    } else {
        let arguments = split_into_args(command);
        if arguments.len() == 1 {
            Command::new(&arguments[0])
                .stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn().map(|process| { *child = process; () })
        } else {
            Command::new(&arguments[0]).args(&arguments[1..])
                .stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn().map(|process| { *child = process; () })
        }
    }
}

pub fn get_command_status(command: &str, uses_shell: bool) -> io::Result<ExitStatus> {
    if uses_shell {
        shell_status(command)
    } else {
        let arguments = split_into_args(command);
        if arguments.len() == 1 {
            Command::new(&arguments[0]).status()
        } else {
            Command::new(&arguments[0]).args(&arguments[1..]).status()
        }
    }
}

#[cfg(windows)]
fn shell_output<S: AsRef<OsStr>>(args: S, child: &mut Child) -> io::Result<()> {
    Command::new("cmd").arg("/C").arg(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().map(|process| {
            *child = process;
            ()
        })
}

#[cfg(windows)]
fn shell_status<S: AsRef<OsStr>>(args: S) -> io::Result<ExitStatus> {
    Command::new("cmd").arg("/C").arg(args).status()
}

#[cfg(not(windows))]
fn shell_output<S: AsRef<OsStr>>(args: S, child: &mut Child) -> io::Result<()> {
    Command::new("sh").arg("-c").arg(args)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map(|process| { *child = process; () })
}

#[cfg(not(windows))]
fn shell_status<S: AsRef<OsStr>>(args: S) -> io::Result<ExitStatus> {
    Command::new("sh").arg("-c").arg(args).status()
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
                    } else {
                        if !buffer.is_empty() {
                            output.push(buffer.clone());
                            buffer.clear();
                        }
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

/// Removes the extension of a given input
fn remove_extension(input: &str) -> &str {
    let mut dir_index = 0;
    let mut ext_index = 0;

    for (id, character) in input.chars().enumerate() {
        if character == '/' { dir_index = id }
        if character == '.' { ext_index = id; }
    }

    // Account for hidden files and directories
    if ext_index == 0 || dir_index + 2 > ext_index { input } else { &input[0..ext_index] }
}

fn basename(input: &str) -> &str {
    let mut index = 0;
    for (id, character) in input.chars().enumerate() {
        if character == '/' { index = id; }
    }
    if index == 0 { input } else { &input[index+1..] }
}

fn dirname(input: &str) -> &str {
    let mut index = 0;
    for (id, character) in input.chars().enumerate() {
        if character == '/' { index = id; }
    }
    if index == 0 { "." } else { &input[0..index] }
}

#[test]
fn path_remove_ext_simple() {
    assert_eq!(remove_extension("foo.txt"), "foo");
}

#[test]
fn path_remove_ext_dir() {
    assert_eq!(remove_extension("dir/foo.txt"), "dir/foo");
}

#[test]
fn path_remove_ext_empty() {
    assert_eq!(remove_extension(""), "");
}

#[test]
fn path_basename_simple() {
    assert_eq!(basename("foo.txt"), "foo.txt");
}

#[test]
fn path_basename_dir() {
    assert_eq!(basename("dir/foo.txt"), "foo.txt");
}

#[test]
fn path_basename_empty() {
    assert_eq!(basename(""), "");
}

#[test]
fn path_dirname_simple() {
    assert_eq!(dirname("foo.txt"), ".");
}

#[test]
fn path_dirname_dir() {
    assert_eq!(dirname("dir/foo.txt"), "dir");
}

#[test]
fn path_dirname_empty() {
    assert_eq!(dirname(""), ".");
}

#[test]
fn build_arguments_test() {
    let tokens = vec![Token::Argument("-i ".to_owned()), Token::Placeholder,
        Token::Argument(" ".to_owned()), Token::RemoveExtension,
        Token::Argument(".mkv".to_owned())];

    let command = ParallelCommand {
        slot_no:   "1",
        job_no:    "1",
        job_total: "1",
        input:     "applesauce.mp4",
        command_template: &tokens,
    };

    let mut arguments = String::new();
    command.build_arguments(&mut arguments);
    assert_eq!(arguments, String::from("-i applesauce.mp4 applesauce.mkv"))
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
