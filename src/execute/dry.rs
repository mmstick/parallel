use super::arguments::{self, InputIteratorErr};
use super::command;
use super::super::tokenizer::Token;
use super::super::input_iterator::InputIterator;

use std::io::{self, Write};

pub fn dry_run(flags: u16, inputs: InputIterator, arguments: &[Token]) {
    let stdout             = io::stdout();
    let mut stdout         = stdout.lock();
    let stderr             = io::stderr();
    let mut stderr         = stderr.lock();
    let mut command_buffer = String::new();
    let slot               = "{SLOT_ID}";
    let job_total          = "{TOTAL_JOBS}";
    let job_id             = "{JOB_ID}";
    let shell              = flags & arguments::SHELL_QUOTE != 0;
    let pipe               = flags & arguments::PIPE_IS_ENABLED != 0;

    for input in inputs {
        match input {
            Ok(input) => {
                let command = command::ParallelCommand {
                    slot_no:          slot,
                    job_no:           job_id,
                    job_total:        job_total,
                    input:            &input,
                    command_template: arguments,
                    flags:            flags,
                };

                command.build_arguments(&mut command_buffer);
                if !pipe {
                    command::append_argument(&mut command_buffer, command.command_template, command.input);
                }
                if shell {
                    let _ = stdout.write(shell_quote(&command_buffer).as_bytes());
                } else {
                    let _ = stdout.write(command_buffer.as_bytes());
                }
                let _ = stdout.write(b"\n");
                command_buffer.clear();
            },
            Err(why) => {
                match why {
                    InputIteratorErr::FileRead(path, why) => {
                        let _ = write!(stderr, "parallel: input file read error: {:?}: {}\n", path, why);
                    },
                }
            }
        }
    }
}

fn shell_quote(command: &str) -> String {
    let mut output = String::with_capacity(command.len() << 1);
    for character in command.chars() {
        match character {
            '$' | ' ' | '\\' | '>' | '<' | '^' | '&' | '#' | '!' | '*' |
            '\'' | '\"' | '`' | '~' | '{' | '}' | '[' | ']' | '(' | ')' |
            ';' | '|' | '?' => output.push('\\'),
            _ => ()
        }
        output.push(character);
    }

    output
}
