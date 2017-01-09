use arguments::{QUIET_MODE, VERBOSE_MODE};
use execute::command::{self, CommandErr};
use input_iterator::InputsLock;
use itoa_array::itoa;
use tokenizer::Token;
use wait_timeout::ChildExt;
use verbose;
use super::pipe::disk::output as pipe_output;
use super::pipe::disk::State;

use std::io::{self, Write};
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Contains all the required data needed for executing commands in parallel.
/// Commands will be generated based on a template of argument tokens combined
/// with the current input argument.
pub struct ExecCommands<'a> {
    pub slot:       usize,
    pub num_inputs: usize,
    pub flags:      u16,
    pub timeout:    Duration,
    pub inputs:     InputsLock,
    pub output_tx:  Sender<State>,
    pub arguments:  &'a [Token],
}

impl<'a> ExecCommands<'a> {
    pub fn run(&mut self) {
        let stdout = io::stdout();
        let stderr = io::stderr();

        let slot               = &self.slot.to_string();
        let mut command_buffer = &mut String::with_capacity(64);
        let has_timeout        = self.timeout != Duration::from_millis(0);
        let mut input          = String::with_capacity(64);
        let mut id_buffer      = [0u8; 64];

        let mut total_buffer   = [0u8; 64];
        let truncate           = itoa(&mut total_buffer, self.num_inputs, 10);
        let job_total          = &total_buffer[0..truncate];

        while let Some((job_id, _)) = self.inputs.try_next(&mut input) {
            if self.flags & VERBOSE_MODE != 0  {
                verbose::processing_task(&stdout, job_id+1, self.num_inputs, &input);
            }

            let truncate = itoa(&mut id_buffer, job_id+1, 10);
            let command = command::ParallelCommand {
                slot_no:          slot,
                job_no:           &id_buffer[0..truncate],
                job_total:        job_total,
                input:            &input,
                command_template: self.arguments,
                flags:            self.flags
            };

            command_buffer.clear();
            match command.exec(command_buffer) {
                Ok(mut child) => {
                    if has_timeout && child.wait_timeout(self.timeout).unwrap().is_none() {
                        let _ = child.kill();
                        pipe_output(&mut child, job_id, input.clone(), &self.output_tx, self.flags & QUIET_MODE != 0);
                    } else {
                        pipe_output(&mut child, job_id, input.clone(), &self.output_tx, self.flags & QUIET_MODE != 0);
                        let _ = child.wait();
                    }
                },
                Err(cmd_err) => {
                    let mut stderr = stderr.lock();
                    let _ = stderr.write(b"parallel: command error: ");
                    let message = match cmd_err {
                        CommandErr::IO(error) => format!("I/O error: {}\n", error),
                    };

                    let _ = stderr.write(message.as_bytes());
                    let message = format!("{}: {}: {}", job_id+1, command.input, message);
                    let _ = self.output_tx.send(State::Error(job_id, message));
                }
            }

            if self.flags & VERBOSE_MODE != 0 {
                verbose::task_complete(&stdout, job_id, self.num_inputs, &input);
            }
        }
    }
}
