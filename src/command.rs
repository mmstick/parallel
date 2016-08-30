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
    pub fn exec(&self, grouped: bool, uses_shell: bool, child: &mut Child)
        -> io::Result<CommandResult>
    {
        // First the arguments will be generated based on the tokens and input.
        let mut arguments = String::with_capacity(self.command_template.len() << 1);
        self.build_arguments(&mut arguments);

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
    fn build_arguments(&self, arguments: &mut String) {
        for arg in self.command_template {
            match *arg {
                Token::Argument(ref arg) => arguments.push_str(arg),
                Token::Basename        => arguments.push_str(basename(self.input)),
                Token::BaseAndExt      => arguments.push_str(basename(remove_extension(self.input))),
                Token::Dirname         => arguments.push_str(dirname(self.input)),
                Token::Job             => arguments.push_str(self.job_no),
                Token::JobTotal        => arguments.push_str(self.job_total),
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
        let mut iter = command.split_whitespace();
        let command = iter.next().unwrap();
        let args = iter.collect::<Vec<&str>>();
        Command::new(command).args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn().map(|process| {
                *child = process;
                ()
            })
    }
}

pub fn get_command_status(command: &str, uses_shell: bool) -> io::Result<ExitStatus> {
    if uses_shell {
        shell_status(command)
    } else {
        let mut iter = command.split_whitespace();
        let command = iter.next().unwrap();
        let args = iter.collect::<Vec<&str>>();
        Command::new(command).args(&args).status()
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().map(|process| {
            *child = process;
            ()
        })
}

#[cfg(not(windows))]
fn shell_status<S: AsRef<OsStr>>(args: S) -> io::Result<ExitStatus> {
    Command::new("sh").arg("-c").arg(args).status()
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
    let input = "applesauce.mp4";
    let job   = "1";
    let slot  = "1";
    let total = "1";
    let tokens = vec![Token::Argument("-i ".to_owned()), Token::Placeholder,
        Token::Argument(" ".to_owned()), Token::RemoveExtension,
        Token::Argument(".mkv".to_owned())];
    let mut arguments = String::new();
    build_arguments(&mut arguments, &tokens, input, slot, job, total);
    assert_eq!(arguments, String::from("-i applesauce.mp4 applesauce.mkv"))
}
