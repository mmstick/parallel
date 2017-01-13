use arguments::{self, JOBLOG};
use execute::command;
use input_iterator::InputsLock;
use shell;
use time::Timespec;
use verbose;
use super::job_log::JobLog;
use super::pipe::disk::State;
use super::child::handle_child;

use std::u16;
use std::time::Duration;
use std::io::{self, Write};
use std::sync::mpsc::Sender;

/// Contains all the required data needed for executing commands in parallel.
/// The inputs will be executed as commands themselves.
pub struct ExecInputs {
    pub num_inputs: usize,
    pub timeout:    Duration,
    pub inputs:     InputsLock,
    pub output_tx:  Sender<State>,
    pub tempdir:    String,
}

impl ExecInputs {
    pub fn run(&mut self, mut flags: u16) {
        let stdout = io::stdout();
        let stderr = io::stderr();

        let has_timeout = self.timeout != Duration::from_millis(0);
        let mut input = String::with_capacity(64);

        while let Some(job_id) = self.inputs.try_next(&mut input) {
            if flags & arguments::VERBOSE_MODE != 0 {
                verbose::processing_task(&stdout, job_id+1, self.num_inputs, &input);
            }

            // Checks the current command to determine if a shell will be required.
            if shell::required(shell::Kind::Input(&input)) {
                flags |= arguments::SHELL_ENABLED;
            } else {
                flags &= u16::MAX ^ arguments::SHELL_ENABLED;
            }

            let (start_time, end_time, exit_value, signal) = match command::get_command_output(&input, flags) {
                Ok(child) => {
                    handle_child(child, &self.output_tx, flags, job_id, input.clone(), has_timeout, self.timeout,
                        &self.tempdir)
                },
                Err(why) => {
                    let mut stderr = stderr.lock();
                    let _ = write!(&mut stderr, "parallel: command error: {}: {}\n", input, why);
                    let message = format!("{}: {}: {}\n", job_id, input, why);
                    let _ = self.output_tx.send(State::Error(job_id, message));
                    (Timespec::new(0, 0), Timespec::new(0, 0), -1, 0)
                }
            };

            if flags & JOBLOG != 0 {
                let runtime = end_time - start_time;
                let _ = self.output_tx.send(State::JobLog(JobLog {
                    job_id:     job_id,
                    start_time: start_time,
                    runtime:    runtime.num_nanoseconds().unwrap_or(0) as u64,
                    exit_value: exit_value,
                    signal:     signal,
                    command:    input.clone(),
                }));
            }

            if flags & arguments::VERBOSE_MODE != 0 {
                verbose::task_complete(&stdout, job_id, self.num_inputs, &input);
            }
        }
    }
}
