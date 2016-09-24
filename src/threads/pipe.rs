use std::io::{BufRead, BufReader, Stdout, Stderr, Write};
use std::process::Child;
use std::sync::mpsc::Sender;
use super::super::arguments::DiskBufferWriter;

/// A `Pipe` may either be a stream from `Stdout` or `Stderr`.
pub enum Pipe {
    Stdout(String),
    Stderr(String, String)
}

/// When using grouped mode, the `State` will tell the program whether the program is still
/// processing, or if it has completed.
pub enum State {
    /// If the program is still processing, this will contain the message received.
    Processing(JobOutput),
    /// The integer supplied with this signal tells the program which process has finished.
    Completed(usize, String),
    /// An error occurred, so the error will be marked.
    Error(usize, String),
}

/// The `JobOutput` structure is utilized when grouping is enabled to transmit a command's
/// associated job ID with it's stdout and stderr buffers back to the main thread to be
/// queued for printing in the order that the inputs are supplied.
pub struct JobOutput {
    pub id:   usize,
    pub pipe: Pipe,
}

impl Pipe {
    /// When a piped message is received and it is to be printed next, print the message
    /// to it's respective buffer.
    pub fn print_message(&self, id: usize, error_file: &mut DiskBufferWriter, stdout: &Stdout,
        stderr: &Stderr)
    {
        let mut stdout = stdout.lock();
        let mut stderr = stderr.lock();
        match *self {
            // The message is meant to be printed on standard output.
            Pipe::Stdout(ref message) => {
                let _ = stdout.write(message.as_bytes());
                let _ = stdout.write(b"\n");
            },
            // The message is meant to be printed on standard error.
            Pipe::Stderr(ref name, ref message) => {
                let _ = stderr.write(message.as_bytes());
                let _ = stderr.write(b"\n");
                if let Err(why) = error_file.write(id.to_string().as_bytes())
                    .and_then(|_| error_file.write(b": "))
                    .and_then(|_| error_file.write(name.as_bytes()))
                    .and_then(|_| error_file.write(b": "))
                    .and_then(|_| error_file.write(message.as_bytes()))
                    .and_then(|_| error_file.write_byte(b'\n'))
                {
                    let _ = stderr.write(b"parallel: I/O error: ");
                    let _ = stderr.write(why.to_string().as_bytes());
                }
            }
        }
    }
}

/// Sends messages received by a `Child` process's standard output and error and sends them
/// to be handled by the grouped output channel.
pub fn output(child: &mut Child, job_id: usize, name: String, output_tx: &Sender<State>, quiet: bool) {
    let stderr = child.stderr.as_mut().expect("unable to open stderr of child");
    let mut stderr_buffer = BufReader::new(stderr).lines();
    if quiet {
        // Only pipe messages from standard error when quiet mode is enabled.
        for stderr in stderr_buffer {
            if let Ok(stderr) = stderr {
                let _ = output_tx.send(State::Processing(JobOutput {
                    id:   job_id,
                    pipe: Pipe::Stderr(name.clone(), stderr)
                }));
            } else {
                break
            }
        }
    } else {
        let stdout = child.stdout.as_mut().expect("unable to open stdout of child");
        let mut stdout_buffer = BufReader::new(stdout).lines();

        // Attempt to read from stdout and stderr simultaneously until both are exhausted of messages.
        loop {
            if let Some(stdout) = stdout_buffer.next() {
                // If a message is received from standard output, it will be sent as a `Pipe::Stdout`.
                if let Ok(stdout) = stdout {
                    let _ = output_tx.send(State::Processing(JobOutput {
                        id:   job_id,
                        pipe: Pipe::Stdout(stdout)
                    }));
                } else {
                    break
                }
            } else if let Some(stderr) = stderr_buffer.next() {
                // If a message is received from standard error, it will be sent as a `Pipe::Stderr`.
                if let Ok(stderr) = stderr {
                    let _ = output_tx.send(State::Processing(JobOutput {
                        id:   job_id,
                        pipe: Pipe::Stderr(name.clone(), stderr)
                    }));
                } else {
                    break
                }
            } else {
                break
            }
        }
    }

    // Signal to the channel that the job has completed.
    let _ = output_tx.send(State::Completed(job_id, name));
}
