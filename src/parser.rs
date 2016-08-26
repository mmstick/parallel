use std::env;
use tokenizer::{Token, tokenize};

pub enum ParseErr {
    JobsNaN(String),
    JobsNoValue,
}

// Parses input arguments and stores their values into their associated variabless.
pub fn parse_arguments(ncores: &mut usize, command: &mut String, arg_tokens: &mut Vec<Token>,
    input_variables: &mut Vec<String>) -> Result<(), ParseErr>
{
    let mut parsing_arguments = true;
    let mut command_is_set    = false;
    let mut raw_args = env::args().skip(1).peekable();
    let mut comm = String::new();
    while let Some(argument) = raw_args.next() {
        if parsing_arguments {
            match argument.as_str() {
                // Defines the number of jobs to run in parallel.
                "-j"  => {
                    match raw_args.peek() {
                        Some(val) => match val.parse::<usize>() {
                            Ok(val) => *ncores = val,
                            Err(_)  => return Err(ParseErr::JobsNaN(val.clone()))
                        },
                        None => return Err(ParseErr::JobsNoValue)
                    }
                    let _ = raw_args.next();
                },
                // Arguments after `:::` are input values.
                ":::" => parsing_arguments = false,
                _ => {
                    if command_is_set {
                        comm.push(' ');
                        comm.push_str(&argument);
                    } else {
                        comm.push_str(&argument);
                        command_is_set = true;
                    }

                }
            }
        } else {
            input_variables.push(argument);
        }
    }

    // This will fill in command and argument information needed by the threads.
    // If there is a space in the argument, then the command has arguments
    match comm.chars().position(|x| x == ' ') {
        Some(pos) => {
            *command    = String::from(&comm[0..pos]);
            *arg_tokens = tokenize(&comm[pos+1..]);
        },
        None => *command = comm
    }

    Ok(())
}
