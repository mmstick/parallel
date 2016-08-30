use std::io::{BufRead, BufReader, Stdout, Stderr, Write};
use std::process::Child;
use std::sync::mpsc::Sender;

/// A `Pipe` may either be a stream from `Stdout` or `Stderr`.
pub enum Pipe {
    Stdout(String),
    Stderr(String)
}

/// When using grouped mode, the `State` will tell the program whether the program is still
/// processing, or if it has completed.
pub enum State {
    /// If the program is still processing, this will contain the message received.
    Processing(JobOutput),
    /// The integer supplied with this signal tells the program which process has finished.
    Completed(usize)
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
    pub fn print_message(&self, stdout: &Stdout, stderr: &Stderr) {
        let mut stdout = stdout.lock();
        let mut stderr = stderr.lock();
        match *self {
            // The message is meant to be printed on standard output.
            Pipe::Stdout(ref message) => {
                let _ = stdout.write(message.as_bytes());
                let _ = stdout.write(b"\n");
            },
            // The message is meant to be printed on standard error.
            Pipe::Stderr(ref message) => {
                let _ = stderr.write(message.as_bytes());
                let _ = stderr.write(b"\n");
            }
        }
    }
}

/// Sends messages received by a `Child` process's standard output and error and sends them
/// to be handled by the grouped output channel.
pub fn output(child: Child, job_id: usize, output_tx: &Sender<State>) {
    // Simultaneously buffer lines from both `stdout` and `stderr`.
    let stdout = child.stdout.expect("unable to open stdout of child");
    let stderr = child.stderr.expect("unable to open stderr of child");
    let mut stdout_buffer = BufReader::new(stdout).lines();
    let mut stderr_buffer = BufReader::new(stderr).lines();

    // Attempt to read from stdout and stderr simultaneously until both are exhausted of messages.
    loop {
        if let Some(stdout) = stdout_buffer.next() {
            // If a message is received from standard output, it will be sent as a `Pipe::Stdout`.
            let _ = stdout.map(|stdout| {
                let _ = output_tx.send(State::Processing(JobOutput {
                    id:   job_id,
                    pipe: Pipe::Stdout(stdout)
                }));
            });
        } else if let Some(stderr) = stderr_buffer.next() {
            // If a message is received from standard error, it will be sent as a `Pipe::Stderr`.
            let _ = stderr.map(|stderr| {
                let _ = output_tx.send(State::Processing(JobOutput {
                    id:   job_id,
                    pipe: Pipe::Stderr(stderr)
                }));
            });
        } else {
            break
        }
    }

    // Signal to the channel that the job has completed.
    let _ = output_tx.send(State::Completed(job_id));
}
