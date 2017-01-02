use arguments;
use command::{self, CommandErr};
use wait_timeout::ChildExt;
use verbose;
use super::pipe::disk::output as pipe_output;
use super::pipe::disk::State;
use super::super::tokenizer::Token;
use super::super::input_iterator::InputIterator;
use super::attempt_next;

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub struct ExecCommands<'a> {
    pub slot:       usize,
    pub num_inputs: usize,
    pub flags:      u8,
    pub delay:      Duration,
    pub timeout:    Duration,
    pub inputs:     Arc<Mutex<InputIterator>>,
    pub output_tx:  Sender<State>,
    pub arguments:  &'a [Token<'a>],
}

impl<'a> ExecCommands<'a> {
    pub fn run(&self) {
        let stdout = io::stdout();
        let stderr = io::stderr();

        let slot               = &self.slot.to_string();
        let job_total          = &self.num_inputs.to_string();
        let mut command_buffer = &mut String::with_capacity(64);
        let has_delay          = self.delay != Duration::from_millis(0);
        let has_timeout        = self.timeout != Duration::from_millis(0);

        while let Some((input, job_id)) = attempt_next(&self.inputs, &stderr, has_delay, self.delay) {
            if self.flags & arguments::VERBOSE_MODE != 0  {
                verbose::processing_task(&stdout, &job_id.to_string(), job_total, &input);
            }

            let command = command::ParallelCommand {
                slot_no:          slot,
                job_no:           &(job_id + 1).to_string(),
                job_total:        job_total,
                input:            &input,
                command_template: self.arguments,
            };

            command_buffer.clear();
            match command.exec(command_buffer, self.flags) {
                Ok(mut child) => {
                    if has_timeout && child.wait_timeout(self.timeout).unwrap().is_none() {
                        let _ = child.kill();
                        pipe_output(&mut child, job_id, input.clone(), &self.output_tx, self.flags & arguments::QUIET_MODE != 0);
                    } else {
                        pipe_output(&mut child, job_id, input.clone(), &self.output_tx, self.flags & arguments::QUIET_MODE != 0);
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
                    let message = format!("{}: {}: {}", command.job_no, command.input, message);
                    let _ = self.output_tx.send(State::Error(job_id, message));
                }
            }

            if self.flags & arguments::VERBOSE_MODE != 0 {
                verbose::task_complete(&stdout, &job_id.to_string(), job_total, &input);
            }
        }
    }
}
