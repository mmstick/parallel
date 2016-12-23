pub mod execute;
pub mod pipe;

use std::io::{stderr, stdout, Write};
use std::path::Path;
use std::sync::mpsc::Receiver;

use super::arguments::Args;
use super::disk_buffer::DiskBuffer;
use self::pipe::State;

pub fn receive_messages(input_rx: Receiver<State>, args: Args, processed_path: &Path, errors_path: &Path) {
    let stdout = stdout();
    let stderr = stderr();

    // Keeps track of which job is currently allowed to print to standard output/error.
    let mut counter = 0;
    // Messages received that are not to be printed will be stored for later use.
    let mut buffer = Vec::new();
    // Store a list of indexes we need to drop from `buffer` after a match has been found.
    let mut drop = Vec::with_capacity(args.ncores);
    // Store a list of completed inputs in the event that the user may need to resume processing.
    let mut processed_file = DiskBuffer::new(processed_path).write().unwrap();
    let mut error_file     = DiskBuffer::new(errors_path).write().unwrap();

    // The loop will only quit once all inputs have been received.
    while counter < args.ninputs {
        // Block and wait until a new buffer is received.
        match input_rx.recv().unwrap() {
            // Signals that the job has completed processing
            State::Completed(job, name) => {
                if job == counter {
                    counter += 1;
                    if let Err(why) = processed_file.write(name.as_bytes()) {
                        let mut stderr = &mut stderr.lock();
                        let _ = write!(stderr, "parallel: I/O error: {}", why);
                    }
                } else {
                    buffer.push(State::Completed(job, name));
                }
            },
            // Signals that an error occurred.
            State::Error(id, message) => {
                if id == counter {
                    counter += 1;
                    if let Err(why) = error_file.write(message.as_bytes()) {
                        let mut stderr = &mut stderr.lock();
                        let _ = write!(stderr, "parallel: I/O error: {}", why);
                    }
                } else {
                    buffer.push(State::Error(id, message));
                }
            }
            // If the received message is a processing signal, there is a message to print.
            State::Processing(output) => {
                if output.id == counter {
                    output.pipe.print_message(output.id, &mut error_file, &stdout, &stderr);
                } else {
                    buffer.push(State::Processing(output));
                }
            }
        }

        // Check to see if there are any stored buffers that can now be printed.
        'outer: loop {
            // Keep track of any changes that have been made in this iteration.
            let mut changed = false;

            // Loop through the list of buffers and print buffers with the next ID in line.
            // If a match was found, `changed` will be set to true and the job added to the
            // drop list. If no change was found, the outer loop will quit.
            for (id, output) in buffer.iter().enumerate() {
                match *output {
                    State::Completed(job, ref name) => {
                        if job == counter {
                            counter += 1;
                            drop.push(id);
                            changed = true;
                            if let Err(why) = processed_file.write(name.as_bytes()) {
                                let mut stderr = &mut stderr.lock();
                                let _ = write!(stderr, "parallel: I/O error: {}", why);
                            }
                            break
                        }
                    },
                    State::Error(job, ref message) => {
                        if job == counter {
                            counter += 1;
                            drop.push(id);
                            changed = true;
                            if let Err(why) = error_file.write(message.as_bytes()) {
                                let mut stderr = &mut stderr.lock();
                                let _ = write!(stderr, "parallel: I/O error: {}", why);
                            }
                            break
                        }
                    }
                    State::Processing(ref output) => {
                        if output.id == counter {
                            output.pipe.print_message(output.id, &mut error_file, &stdout, &stderr);
                            changed = true;
                            drop.push(id);
                        }
                    }
                }
            }

            // Drop the buffers that were used.
            if !drop.is_empty() { drop_used_values(&mut buffer, &mut drop); }

            // If no change is made during a loop, it's time to give up searching.
            if !changed { break 'outer }
        }
    }

    if let Err(why) = processed_file.flush() {
        let mut stderr = &mut stderr.lock();
        let _ = write!(stderr, "parallel: I/O error: {}", why);
    }

    if let Err(why) = error_file.flush() {
        let mut stderr = &mut stderr.lock();
        let _ = write!(stderr, "parallel: I/O error: {}", why);
    }
}

fn drop_used_values(buffer: &mut Vec<State>, drop: &mut Vec<usize>) {
    drop.sort();
    for id in drop.drain(0..).rev() {
        let _ = buffer.remove(id);
    }
    buffer.shrink_to_fit()
}
