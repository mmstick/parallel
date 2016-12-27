use arguments::{self, InputIteratorErr};
use command::{self, CommandErr};
use super::pipe::disk::output as pipe_output;
use super::pipe::disk::State;
use super::super::tokenizer::Token;
use super::super::input_iterator::InputIterator;
use verbose;
use std::io::{self, Write, Stderr};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

// Attempts to obtain the next input argument along with it's job ID from the `InputIterator`.
// NOTE: Some reason this halves the wall time compared to making this a method of `InputIterator`.
fn attempt_next(inputs: &Arc<Mutex<InputIterator>>, stderr: &Stderr) -> Option<(String, usize)> {
    let mut inputs = inputs.lock().unwrap();
    let job_id = inputs.curr_argument;
    match inputs.next() {
        None            => None,
        Some(Ok(input)) => Some((input, job_id)),
        Some(Err(why))  => {
            let stderr = &mut stderr.lock();
            match why {
                InputIteratorErr::FileRead(path, why) => {
                    let _ = write!(stderr, "parallel: input file read error: {:?}: {}\n", path, why);
                },
            }
            None
        }
    }
}

/// Builds and executes commands based on a provided template and associated inputs.
pub fn command(slot: usize, num_inputs: usize, flags: u8, arguments: &[Token],
    inputs: Arc<Mutex<InputIterator>>, output_tx: Sender<State>)
{
    let stdout = io::stdout();
    let stderr = io::stderr();

    let slot      = &slot.to_string();
    let job_total = &num_inputs.to_string();
    let mut command_buffer = &mut String::with_capacity(64);

    while let Some((input, job_id)) = attempt_next(&inputs, &stderr) {
        if flags & arguments::VERBOSE_MODE != 0  {
            verbose::processing_task(&stdout, &job_id.to_string(), job_total, &input);
        }

        let command = command::ParallelCommand {
            slot_no:          slot,
            job_no:           &job_id.to_string(),
            job_total:        job_total,
            input:            &input,
            command_template: arguments,
        };

        command_buffer.clear();
        match command.exec(command_buffer, flags) {
            Ok(mut child) => {
                pipe_output(&mut child, job_id, input.clone(), &output_tx, flags & arguments::QUIET_MODE != 0);
                let _ = child.wait();
            },
            Err(cmd_err) => {
                let mut stderr = stderr.lock();
                let _ = stderr.write(b"parallel: command error: ");
                let message = match cmd_err {
                    CommandErr::IO(error) => format!("I/O error: {}\n", error),
                };

                let _ = stderr.write(message.as_bytes());
                let message = format!("{}: {}: {}", command.job_no, command.input, message);
                let _ = output_tx.send(State::Error(job_id, message));
            }
        }

        if flags & arguments::VERBOSE_MODE != 0 {
            verbose::task_complete(&stdout, &job_id.to_string(), job_total, &input);
        }
    }
}

/// Executes inputs as commands
pub fn inputs(num_inputs: usize, flags: u8, inputs: Arc<Mutex<InputIterator>>, output_tx: Sender<State>) {
    let stdout = io::stdout();
    let stderr = io::stderr();

    let job_total = &num_inputs.to_string();

    while let Some((input, job_id)) = attempt_next(&inputs, &stderr) {
        if flags & arguments::VERBOSE_MODE != 0 {
            verbose::processing_task(&stdout, &job_id.to_string(), job_total, &input);
        }

        match command::get_command_output(&input, flags) {
            Ok(mut child) => {
                pipe_output(&mut child, job_id, input.clone(), &output_tx, flags & arguments::QUIET_MODE != 0);
                let _ = child.wait();
            },
            Err(why) => {
                let mut stderr = stderr.lock();
                let _ = write!(&mut stderr, "parallel: command error: {}: {}\n", input, why);
                let message = format!("{}: {}: {}\n", job_id, input, why);
                let _ = output_tx.send(State::Error(job_id, message));
            }
        }

        if flags & arguments::VERBOSE_MODE != 0 {
            verbose::task_complete(&stdout, &job_id.to_string(), job_total, &input);
        }
    }
}
