use arguments::{VERBOSE_MODE, JOBLOG};
use execute::command::{self, CommandErr};
use input_iterator::InputsLock;
use numtoa::NumToA;
use time::{self, Timespec};
use tokenizer::Token;
use verbose;
use super::pipe::disk::State;
use super::job_log::JobLog;
use super::child::handle_child;

use std::io::{self, Read, Write};
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Contains all the required data needed for executing commands in parallel.
/// Commands will be generated based on a template of argument tokens combined
/// with the current input argument.
pub struct ExecCommands<IO: Read> {
    pub slot:       usize,
    pub num_inputs: usize,
    pub flags:      u16,
    pub timeout:    Duration,
    pub inputs:     InputsLock<IO>,
    pub output_tx:  Sender<State>,
    pub arguments:  &'static [Token],
    pub tempdir:    String,
}

impl<IO: Read> ExecCommands<IO> {
    pub fn run(&mut self) {
        let stdout = io::stdout();
        let stderr = io::stderr();

        let slot               = &self.slot.to_string();
        let mut command_buffer = &mut String::with_capacity(64);
        let has_timeout        = self.timeout != Duration::from_millis(0);
        let mut input          = String::with_capacity(64);
        let mut id_buffer      = [0u8; 20];
        let mut job_buffer     = [0u8; 20];
        let mut total_buffer   = [0u8; 20];
        let mut start_indice   = self.num_inputs.numtoa(10, &mut total_buffer);
        let job_total          = &total_buffer[start_indice..];


        while let Some(job_id) = self.inputs.try_next(&mut input) {
            if self.flags & VERBOSE_MODE != 0  {
                verbose::processing_task(&stdout, job_id+1, self.num_inputs, &input);
            }

            start_indice = (job_id+1).numtoa(10, &mut id_buffer);
            let command = command::ParallelCommand {
                slot_no:          slot,
                job_no:           &id_buffer[start_indice..],
                job_total:        job_total,
                input:            &input,
                command_template: self.arguments,
                flags:            self.flags
            };

            command_buffer.clear();
            let (start_time, end_time, exit_value, signal) = match command.exec(command_buffer) {
                Ok(child) => {
                    handle_child(child, &self.output_tx, self.flags, job_id, input.clone(), has_timeout, self.timeout,
                        &self.tempdir, &mut job_buffer)
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
                    (Timespec::new(0, 0), Timespec::new(0, 0), -1, 0)
                }
            };

            if self.flags & JOBLOG != 0 {
                let runtime: time::Duration = end_time - start_time;
                let _ = self.output_tx.send(State::JobLog(JobLog {
                    job_id:     job_id,
                    start_time: start_time,
                    runtime:    runtime.num_nanoseconds().unwrap_or(0) as u64,
                    exit_value: exit_value,
                    signal:     signal,
                    flags:      self.flags,
                    command:    command_buffer.clone(),
                }));
            }

            if self.flags & VERBOSE_MODE != 0 {
                verbose::task_complete(&stdout, job_id, self.num_inputs, &input);
            }
        }
    }
}
