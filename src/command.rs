use std::io::{StderrLock, Write};
use std::process::{Command, Output};
use tokenizer::Token;

pub enum CommandErr {
    Failed(String, Vec<String>, String)
}

impl CommandErr {
    pub fn handle(self, stderr: &mut StderrLock) {
        let _ = stderr.write(b"parallel: command error: ");
        match self {
            CommandErr::Failed(command, arguments, error) => {
                let _ = stderr.write(command.as_bytes());
                for arg in &arguments {
                    let _ = stderr.write(b" ");
                    let _ = stderr.write(arg.as_bytes());
                }
                let _ = stderr.write(b": ");
                let _ = stderr.write(error.as_bytes());
                let _ = stderr.write(b"\n");
            }
        }
    }
}

/// Builds the command and executes it
pub fn exec(input: &str, command: &str, arg_tokens: &[Token], slot_id: &str, job_id: &str,
    job_total :&str, grouping: bool) -> Result<Option<Output>, CommandErr>
{
    // First the arguments will be generated based on the tokens and input.
    let mut arguments = Vec::new();
    build_arguments(&mut arguments, arg_tokens, input, slot_id, job_id, job_total);

    // Check to see if any placeholder tokens are in use.
    let placeholder_exists = arg_tokens.iter().any(|ref x| {
        x == &&Token::BaseAndExt || x == &&Token::Basename || x == &&Token::Dirname ||
        x == &&Token::Job || x == &&Token::Placeholder || x == &&Token::RemoveExtension ||
        x == &&Token::Slot
    });

    // If no placeholder tokens are in use, the user probably wants to infer one.
    if !placeholder_exists {
        arguments.push(String::from(input));
    }

    if grouping {
        Command::new(&command).args(&arguments).output()
            .map(|x| Some(x)).map_err(|why| {
                CommandErr::Failed(String::from(command), arguments, why.to_string())
            })
    } else {
        match Command::new(&command).args(&arguments).status() {
            Ok(_)    => Ok(None),
            Err(why) => Err(CommandErr::Failed(String::from(command), arguments, why.to_string()))
        }
    }
}

/// Builds arguments using the `tokens` template with the current `input` value.
/// The arguments will be stored within a `Vec<String>`
fn build_arguments(args_vec: &mut Vec<String>, tokens: &[Token], input: &str, slot: &str,
    job: &str, job_total: &str)
{
    let mut arguments = String::new();
    for arg in tokens {
        match *arg {
            Token::Character(arg)  => arguments.push(arg),
            Token::Basename        => arguments.push_str(basename(input)),
            Token::BaseAndExt      => arguments.push_str(basename(remove_extension(input))),
            Token::Dirname         => arguments.push_str(dirname(input)),
            Token::Job             => arguments.push_str(job),
            Token::JobTotal        => arguments.push_str(job_total),
            Token::Placeholder     => arguments.push_str(input),
            Token::RemoveExtension => arguments.push_str(remove_extension(input)),
            Token::Slot            => arguments.push_str(slot)
        }
    }

    for argument in arguments.split_whitespace() {
        args_vec.push(String::from(argument));
    }
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
    if ext_index == 0 || dir_index > ext_index || dir_index == ext_index - 1 {
        input
    } else {
        &input[0..ext_index]
    }
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
    let tokens = vec![
        Token::Character('-'), Token::Character('i'), Token::Character(' '), Token::Placeholder,
        Token::Character(' '), Token::RemoveExtension, Token::Character('.'), Token::Character('m'),
        Token::Character('k'),Token::Character('v')
    ];
    let mut arguments = Vec::new();
    build_arguments(&mut arguments, &tokens, input, slot, job, total);
    let expected = vec![
        String::from("-i"), String::from("applesauce.mp4"), String::from("applesauce.mkv")
    ];
    assert_eq!(arguments, expected)
}
