use super::arguments::{self, InputIteratorErr};
use super::command;
use super::super::tokenizer::Token;
use super::super::input_iterator::InputIterator;

use std::io::{self, Write};

pub fn dry_run(flags: u16, inputs: InputIterator, arguments: &[Token]) {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    let mut command_buffer = String::new();
    let slot      = "{SLOT_ID}";
    let job_total = "{TOTAL_JOBS}";
    let job_id    = "{JOB_ID}";

    for input in inputs {
        match input {
            Ok(input) => {
                let command = command::ParallelCommand {
                    slot_no:          slot,
                    job_no:           job_id,
                    job_total:        job_total,
                    input:            &input,
                    command_template: arguments,
                };

                let pipe_enabled = flags & arguments::PIPE_IS_ENABLED != 0;
                command.build_arguments(&mut command_buffer, pipe_enabled);
                if !pipe_enabled {
                    command::append_argument(&mut command_buffer, command.command_template, command.input);
                }
                let _ = writeln!(stdout, "{}", command_buffer);
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
