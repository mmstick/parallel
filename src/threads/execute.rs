use arguments::{Flags, InputIterator, InputIteratorErr};
use command::{self, CommandErr};
use super::pipe::{self, State};
use super::super::arguments::tokenizer::Token;
use verbose;

use std::io::{self, Write, Stderr};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

// Attempts to obtain the next input argument along with it's job ID from the `InputIterator`.
// NOTE: Some reason this halves the wall time compared to making this a method of `InputIterator`.
fn attempt_next(inputs: &Arc<Mutex<InputIterator>>, stderr: &Stderr) -> Option<(String, usize)> {
    let mut inputs = inputs.lock().unwrap();
    let job_id = inputs.curr_argument;
    let input: String = match inputs.next() {
        None            => return None,
        Some(Ok(input)) => input,
        Some(Err(why))  => {
            let stderr = &mut stderr.lock();
            match why {
                InputIteratorErr::FileRead(path, why) => {
                    let _ = write!(stderr, "parallel: input file read error: {:?}: {}\n", path, why);
                },
            }
            return None;
        }
    };
    Some((input, job_id))
}

/// Builds and executes commands based on a provided template and associated inputs.
pub fn command(slot: usize, num_inputs: usize, flags: Flags, arguments: Vec<Token>,
    inputs: Arc<Mutex<InputIterator>>, output_tx: Sender<State>)
{
    let stdout = io::stdout();
    let stderr = io::stderr();

    let slot = slot.to_string();
    let job_total = num_inputs.to_string();

    while let Some((input, job_id)) = attempt_next(&inputs, &stderr) {
        if flags.verbose {
            verbose::processing_task(&stdout, &job_id.to_string(), &job_total, &input);
        }

        let command = command::ParallelCommand {
            slot_no:          &slot,
            job_no:           &job_id.to_string(),
            job_total:        &job_total,
            input:            &input,
            command_template: &arguments,
        };

        match command.exec(flags.grouped, flags.uses_shell, flags.quiet) {
            Ok(command::CommandResult::Grouped(mut child)) => {
                pipe::output(&mut child, job_id, input.clone(), &output_tx, flags.quiet);
                let _ = child.wait();
            },
            Ok(_) => (),
            Err(cmd_err) => {
                let mut stderr = stderr.lock();
                let _ = stderr.write(b"parallel: command error: ");
                let message = match cmd_err {
                    CommandErr::IO(error) => format!("I/O error: {}\n", error),
                    CommandErr::Input(error) => match error {
                        InputIteratorErr::FileRead(path, why) => {
                            format!("input file read error: {:?}: {}\n", path, why)
                        },
                    }
                };

                let _ = stderr.write(message.as_bytes());
                let message = format!("{}: {}: {}", command.job_no, command.input, message);
                let _ = output_tx.send(State::Error(job_id, message));
            }
        }

        if flags.verbose {
            verbose::task_complete(&stdout, &job_id.to_string(), &job_total, &input);
        }
    }
}

/// Executes inputs as commands
pub fn inputs(num_inputs: usize, flags: Flags, inputs: Arc<Mutex<InputIterator>>,
    output_tx: Sender<State>) {
    let stdout = io::stdout();
    let stderr = io::stderr();

    let job_total = num_inputs.to_string();

    while let Some((input, job_id)) = attempt_next(&inputs, &stderr) {
        if flags.verbose {
            verbose::processing_task(&stdout, &job_id.to_string(), &job_total, &input);
        }

        if flags.grouped {
            match command::get_command_output(&input, flags.uses_shell, flags.quiet) {
                Ok(mut child) => {
                    pipe::output(&mut child, job_id, input.clone(), &output_tx, flags.quiet);
                    let _ = child.wait();
                },
                Err(why) => {
                    let mut stderr = stderr.lock();
                    let _ = write!(&mut stderr, "parallel: command error: {}: {}\n",
                        input, why);
                    let message = format!("{}: {}: {}\n", job_id, input, why);
                    let _ = output_tx.send(State::Error(job_id, message));
                }
            }
        } else if let Err(why) = command::get_command_status(&input, flags.uses_shell, flags.quiet) {
            let mut stderr = stderr.lock();
            let _ = stderr.write(b"parallel: command error:");
            let _ = write!(&mut stderr, "{}: {}\n", input, why);
            let message = format!("{}: {}: {}\n", job_id, input, why);
            let _ = output_tx.send(State::Error(job_id, message));
        }

        if flags.verbose {
            verbose::task_complete(&stdout, &job_id.to_string(), &job_total, &input);
        }
    }
}
