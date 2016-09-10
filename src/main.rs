extern crate num_cpus;
extern crate permutate;
mod arguments; // Collects the input arguments given to the program.
mod command;   // Contains the functionality for building and processing external commands.
mod pipe;      // Used for piping the outputs in grouped mode so that they are ordered.
mod tokenizer; // Takes the command template that is provided and reduces it to digestible tokens.
mod verbose;   // Handles printing of verbose messages.

use std::io::{self, Write};
use std::process::Child;
use std::mem; // Gain access to mem::uninitialized::<T>()
use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};

use arguments::{Args, Flags};
use tokenizer::Token;
use pipe::State;

fn main() {
    // Obtain a handle to standard error's buffer so we can write directly to it.
    let stdout = io::stdout();
    let stderr = io::stderr();

    // Set the default arguments for the application.
    let mut args = Args::new();

    // Let's collect all parameters that we need from the program's arguments.
    // If an error is returned, this will handle that error as efficiently as possible.
    if let Err(error) = args.parse() { error.handle(stdout, stderr); }
    if args.flags.verbose {
        verbose::total_inputs(&stdout, args.ncores, args.ninputs);
    }

    // Keeps track of the current step in the input queue.
    // Values are stored in an `Arc` to allow sharing with multiple threads.
    let shared_counter = Arc::new(AtomicUsize::new(0));
    let shared_input = Arc::new(args.inputs);

    // If grouping is enabled, stdout and stderr will be buffered.
    let (output_tx, input_rx) = channel::<State>();

    // First we will create as many threads as `ncores` specifies.
    // The `threads` vector will contain the thread handles needed to
    // know when to quit the program.
    let mut threads: Vec<JoinHandle<()>> = Vec::with_capacity(args.ncores);

    // The `slot` variable is required by the {%} token.
    for slot in 1..args.ncores+1 {
        // Allow the thread to gain access to the list of inputs.
        let inputs = shared_input.clone();
        // Allow the thread to access the input counter
        let counter = shared_counter.clone();
        // Allow the thread to know when it's time to stop.
        let num_inputs = args.ninputs;
        // The boolean flags that will control the behavior of the threads.
        let flags = args.flags.clone();
        // Each thread will receive it's own sender for sending stderr/stdout buffers.
        let output_tx = output_tx.clone();

        let handle: JoinHandle<()> = if flags.inputs_are_commands {
            thread::spawn(move || {
                exec_inputs_as_commands(num_inputs, flags, inputs, counter, output_tx);
            })
        } else {
            // Create a local copy of the tokens for the thread.
            let arguments = args.arguments.clone();
            thread::spawn(move || {
                exec_commands(slot, num_inputs, flags, arguments, inputs, counter, output_tx);
            })
        };

        // After the thread has been created, add the important pieces needed by the
        // main thread to the `threads` vector.
        threads.push(handle);
    }

    if args.flags.grouped {
        // Keeps track of which job is currently allowed to print to standard output/error.
        let mut counter = 1;
        // Messages received that are not to be printed will be stored for later use.
        let mut buffer = Vec::new();
        // Store a list of indexes we need to drop from `buffer` after a match has been found.
        let mut drop = Vec::with_capacity(args.ncores);

        // The loop will only quit once all inputs have been received. I guarantee it.
        while counter != args.ninputs + 1 {
            // Block and wait until a new buffer is received.
            match input_rx.recv().unwrap() {
                // Signals that the job has completed processing
                State::Completed(job) => {
                    if job == counter {
                        counter += 1;
                    } else {
                        buffer.push(State::Completed(job));
                    }
                },
                // If the received message is a processing signal, there is a message to print.
                State::Processing(output) => {
                    if output.id == counter {
                        output.pipe.print_message(&stdout, &stderr);
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
                        State::Completed(job) => {
                            if job == counter {
                                counter += 1;
                                drop.push(id);
                                changed = true;
                                break
                            }
                        },
                        State::Processing(ref output) => {
                            if output.id == counter {
                                output.pipe.print_message(&stdout, &stderr);
                                changed = true;
                                drop.push(id);
                            }
                        }
                    }
                }

                // Drop the buffers that were used.
                if !drop.is_empty() {
                    // Values have to be dropped in reverse because each time a value is
                    // removed from a vector, all of the items to the right are shifted to
                    // to the left.
                    drop.sort();
                    for id in drop.drain(0..).rev() {
                        let _ = buffer.remove(id);
                    }
                }

                // If no change is made during a loop, it's time to give up searching.
                if !changed { break 'outer }
            }
        }
    }

    // Wait for each thread to complete before quitting the program.
    for thread in threads.into_iter() { thread.join().unwrap(); }
}

fn exec_commands(slot: usize, num_inputs: usize, flags: Flags, arguments: Vec<Token>,
    inputs: Arc<Vec<String>>, counter: Arc<AtomicUsize>, output_tx: Sender<State>)
{
    let stdout = io::stdout();
    let stderr = io::stderr();

    let slot = slot.to_string();
    let job_total = num_inputs.to_string();

    while let Ok((input, job_id)) = next_job(&counter, &inputs, num_inputs) {
        if flags.verbose {
            verbose::processing_task(&stdout, &job_id.to_string(), &job_total, input);
        }

        let command = command::ParallelCommand {
            slot_no:          &slot,
            job_no:           &job_id.to_string(),
            job_total:        &job_total,
            input:            input,
            command_template: &arguments,
        };

        // This `child` handle will allow us to pipe the standard output and error.
        // We can guarantee this to be safe because the value will only be
        // used on the condition that this value is set.
        let mut child = unsafe { mem::uninitialized::<Child>() };
        match command.exec(flags.grouped, flags.uses_shell, flags.quiet, &mut child, &inputs) {
            Ok(command::CommandResult::Grouped) => {
                pipe::output(child, job_id, &output_tx, flags.quiet)
            },
            Ok(_) => (),
            Err(cmd_err) => {
                let mut stderr = stderr.lock();
                let _ = stderr.write(b"parallel: command error: ");
                let _ = stderr.write(cmd_err.to_string().as_bytes());
                let _ = stderr.write(b"\n");
            }
        }

        if flags.verbose {
            verbose::task_complete(&stdout, &job_id.to_string(), &job_total, input);
        }
    }
}

fn exec_inputs_as_commands(num_inputs: usize, flags: Flags, inputs: Arc<Vec<String>>,
    counter: Arc<AtomicUsize>, output_tx: Sender<State>) {
    let stdout = io::stdout();
    let stderr = io::stderr();

    let job_total = num_inputs.to_string();

    while let Ok((input, job_id)) = next_job(&counter, &inputs, num_inputs) {
        if flags.verbose {
            verbose::processing_task(&stdout, &job_id.to_string(), &job_total, input);
        }

        if flags.grouped {
            // We can guarantee this to be safe because the value will only be
            // used on the condition that this value is set.
            let mut child = unsafe { mem::uninitialized::<Child>() };
            match command::get_command_output(input, flags.uses_shell, flags.quiet, &mut child) {
                Ok(_) => pipe::output(child, job_id, &output_tx, flags.quiet),
                Err(why) => {
                    let mut stderr = stderr.lock();
                    let _ = write!(&mut stderr, "parallel: command error: {}: {}\n",
                        input, why);
                }
            }
        } else if let Err(why) = command::get_command_status(input, flags.uses_shell, flags.quiet) {
            let mut stderr = stderr.lock();
            let _ = stderr.write(b"parallel: command error:");
            let _ = write!(&mut stderr, "{}: {}\n", input, why);
        }

        if flags.verbose {
            verbose::task_complete(&stdout, &job_id.to_string(), &job_total, input);
        }
    }
}

/// Increments the counter and returns the next value with it's associated job ID.
fn next_job<'a>(counter: &Arc<AtomicUsize>, inputs: &'a [String], num_inputs: usize)
    -> Result<(&'a str, usize), ()>
{
    let counter = counter.fetch_add(1, Ordering::SeqCst);
    if counter >= num_inputs { Err(()) } else { Ok((&inputs[counter], (counter + 1))) }
}
