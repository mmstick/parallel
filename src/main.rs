extern crate num_cpus;
extern crate permutate;
mod arguments; // Collects the input arguments given to the program.
mod command;   // Contains the functionality for building and processing external commands.
mod pipe;      // Used for piping the outputs in grouped mode so that they are ordered.
mod tokenizer; // Takes the command template that is provided and reduces it to digestible tokens.
mod verbose;   // Handles printing of verbose messages.

use std::io::{self, Write};
use std::process::{exit, Child};
use std::mem; // Gain access to mem::uninitialized::<T>()
use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;

use arguments::{Args, ParseErr};
use pipe::State;

/* TODO: Possible features that could be integrated:
 - Benchmarking
 - SSH support
*/

fn main() {
    // Obtain a handle to standard error's buffer so we can write directly to it.
    let stdout = io::stdout();
    let stderr = io::stderr();

    // Set the default arguments for the application.
    let mut args = Args {
        ncores: num_cpus::get(),
        grouped: true,
        uses_shell: true,
        verbose: false,
        inputs_are_commands: false,
        arguments: Vec::new(),
        inputs: Vec::new()
    };

    // Let's collect all parameters that we need from the program's arguments.
    // If an error is returned, this will handle that error as efficiently as possible.
    if let Err(why) = args.parse() {
        // Always lock an output buffer before using it.
        let mut stderr = stderr.lock();
        let mut stdout = stdout.lock();
        let _ = stderr.write(b"parallel: parsing error: ");
        match why {
            ParseErr::InputFileError(file, why) => {
                let _ = write!(&mut stderr, "unable to open {}: {}\n", file, why);
            },
            ParseErr::JobsNaN(value) => {
                let _ = write!(&mut stderr, "jobs parameter, '{}', is not a number.\n", value);
            },
            ParseErr::JobsNoValue => {
                let _ = stderr.write(b"no jobs parameter was defined.\n");
            },
            ParseErr::InvalidArgument(argument) => {
                let _ = write!(&mut stderr, "invalid argument: {}\n", argument);
            },
            ParseErr::NoArguments => {
                let _ = write!(&mut stderr, "no input arguments were given.\n");
            }
        };
        let _ = stdout.write(b"For help on command-line usage, execute `parallel -h`\n");
        exit(1);
    }

    // It will be useful to know the number of inputs, to know when to quit.
    let num_inputs = args.inputs.len();

    // Keeps track of the current step in the input queue.
    // All threads will share this counter without stepping on each other's toes.
    let shared_counter = Arc::new(AtomicUsize::new(0));

    // We will share the same list of inputs with each thread.
    let shared_input = Arc::new(args.inputs);

    // If grouping is enabled, stdout and stderr will be buffered.
    let (output_tx, input_rx) = channel::<State>();

    // First we will create as many threads as `ncores` specifies.
    // The `threads` vector will contain the thread handles needed to
    // know when to quit the program.
    let mut threads: Vec<JoinHandle<()>> = Vec::with_capacity(args.ncores);

    if args.verbose {
        verbose::total_inputs(&io::stdout(), args.ncores, num_inputs);
    }

    // The `slot` variable is required by the {%} token.
    for slot in 1..args.ncores+1 {
        // The base command template that each thread will use.
        let arguments = args.arguments.clone();
        // Allow the thread to gain access to the list of inputs.
        let inputs = shared_input.clone();
        // Allow the thread to access the input counter
        let counter = shared_counter.clone();
        // Allow the thread to know when it's time to stop.
        let num_inputs = num_inputs;
        // If grouped is set to true, stdout/stderr buffers will be collected.
        let grouped = args.grouped;
        // If `uses_shell` is set to true, commands will be executed in the platform's shell.
        let uses_shell = args.uses_shell;
        // If set to true, this will print the current processing task.
        let verbose_enabled = args.verbose;
        // If no command arguments were given, then the inputs will be read as commands.
        let inputs_are_commands = args.inputs_are_commands;
        // Each thread will receive it's own sender for sending stderr/stdout buffers.
        let output_tx = output_tx.clone();

        // The actual thread where the work will happen on incoming data.
        let handle: JoinHandle<()> = thread::spawn(move || {
            // Grab a handle to standard output/error for this thread.
            let stdout = io::stdout();
            let stderr = io::stderr();

            // The {%} token contains the thread's ID.
            let slot = slot.to_string();
            // The {#^} token contains the total number of jobs.
            let job_total = num_inputs.to_string();

            // This will only break when all inputs are processed.
            loop {
                // Obtain the Nth item and it's job ID from the list of inputs.
                let (input, job_id) = {
                    // Atomically increment the counter
                    let counter = counter.fetch_add(1, Ordering::SeqCst);
                    // Check to see if all inputs have already been processed
                    if counter >= num_inputs { break } else { (&inputs[counter], (counter + 1)) }
                };

                if verbose_enabled {
                    verbose::processing_task(&stdout, &job_id.to_string(), &job_total, input);
                }

                if inputs_are_commands {
                    if grouped {
                        // We can guarantee this to be safe because the value will only be
                        // used on the condition that this value is set.
                        let mut child = unsafe { mem::uninitialized::<Child>() };

                        // Executes each input as if it were a command and returns a
                        // `Command::Output`, if the command executes successfully.
                        match command::get_command_output(input, uses_shell, &mut child) {
                            Ok(_) => pipe::output(child, job_id, &output_tx),
                            // The command has, sadly, failed. This will tell the user why.
                            Err(why) => {
                                let mut stderr = stderr.lock();
                                let _ = write!(&mut stderr, "parallel: command error: {}: {}\n",
                                    input, why);
                            }
                        }
                    } else {
                        // With no need to group the outputs, we only need to know the status
                        // of the command's execution. The standard output and standard error
                        // will automatically be inherited.
                        if let Err(why) = command::get_command_status(input, uses_shell) {
                            let mut stderr = stderr.lock();
                            let _ = stderr.write(b"parallel: command error:");
                            let _ = write!(&mut stderr, "{}: {}\n", input, why);
                        }
                    }
                } else {
                    // Build a command by merging the command template with the input,
                    // and then execute that command.
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

                    match command.exec(grouped, uses_shell, &mut child, &inputs ) {
                        // If grouping is enabled, then we have an output to process.
                        Ok(command::CommandResult::Grouped) => {
                            pipe::output(child, job_id, &output_tx)
                        },
                        // If grouping was not enabled, nothing was returned.
                        Ok(_) => (),
                        // I've an error handler already created for this error type.
                        Err(cmd_err) => {
                            let mut stderr = stderr.lock();
                            let _ = stderr.write(b"parallel: command error: ");
                            let _ = stderr.write(cmd_err.to_string().as_bytes());
                            let _ = stderr.write(b"\n");
                        }
                    }
                }

                if verbose_enabled {
                    verbose::task_complete(&stdout, &job_id.to_string(), &job_total, input);
                }
            }
        });

        // After the thread has been created, add the important pieces needed by the
        // main thread to the `threads` vector.
        threads.push(handle);
    }

    if args.grouped {
        // Keeps track of which job is currently allowed to print to standard output/error.
        let mut counter = 1;
        // Messages received that are not to be printed will be stored for later use.
        let mut buffer = Vec::new();
        // Store a list of indexes we need to drop from `buffer` after a match has been found.
        let mut drop = Vec::with_capacity(args.ncores);

        // The loop will only quit once all inputs have been received. I guarantee it.
        while counter != num_inputs + 1 {
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
                    match output {
                        &State::Completed(job) => {
                            if job == counter {
                                counter += 1;
                                drop.push(id);
                                changed = true;
                                break
                            }
                        },
                        &State::Processing(ref output) => {
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
                    // removed from a vector, all of them items to the right are shifted to
                    // to he left.
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
