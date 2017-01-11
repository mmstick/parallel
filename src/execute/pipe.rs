pub mod disk {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::process::Child;
    use std::sync::mpsc::Sender;
    use filepaths;
    use super::super::job_log::JobLog;

    /// When using grouped mode, the `State` will tell the program whether the program is still
    /// processing, or if it has completed.
    pub enum State {
        /// The integer supplied with this signal tells the program which process has finished.
        Completed(usize, String),
        /// An error occurred, so the error will be marked.
        Error(usize, String),
        /// (job_id, start_time, runtime, exit_value, signal, command)
        JobLog(JobLog),
    }

    /// Sends messages received by a `Child` process's standard output and error and sends them
    /// to be handled by the grouped output channel.
    pub fn output(child: &mut Child, job_id: usize, name: String, output_tx: &Sender<State>, quiet: bool) {
        let (_, stdout_path, stderr_path) = filepaths::new_job(job_id);
        let mut stdout_file = File::create(stdout_path).expect("unable to create job stdout file");
        let mut stderr_file = File::create(stderr_path).expect("unable to create job stderr file");

        let stderr = child.stderr.as_mut().expect("unable to open stderr of child");
        let mut membuffer = [0u8; 8 * 1024];
        if quiet {
            // Only pipe messages from standard error when quiet mode is enabled.
            while let Ok(bytes_read) = stderr.read(&mut membuffer[..]) {
                if bytes_read != 0 {
                    let _ = stderr_file.write(&membuffer[0..bytes_read]);
                } else {
                    break
                }
            }
        } else {
            let mut stdout = child.stdout.as_mut().expect("unable to open stdout of child");

            // Attempt to read from stdout and stderr simultaneously until both are exhausted of messages.
            loop {
                if let Ok(bytes_read) = stdout.read(&mut membuffer[..]) {
                    if bytes_read != 0 {
                        let _ = stdout_file.write(&membuffer[0..bytes_read]);
                    } else if let Ok(bytes_read) = stderr.read(&mut membuffer[..]) {
                        if bytes_read != 0 {
                            let _ = stderr_file.write(&membuffer[0..bytes_read]);
                        } else {
                            break
                        }
                    }
                } else if let Ok(bytes_read) = stderr.read(&mut membuffer[..]) {
                    if bytes_read != 0 {
                        let _ = stderr_file.write(&membuffer[0..bytes_read]);
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
}
