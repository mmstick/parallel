extern crate num_cpus;
mod command;   // Contains the functionality for building and processing external commands.
mod tokenizer; // Takes the command template that is provided and reduces it to digestible tokens.
mod arguments; // Collects the input arguments given to the program.

use std::io::{self, Write};
use std::process::exit;

use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;

use arguments::{Args, ParseErr};

/* TODO: Functionality can be increased to accept the following syntaxes from GNU Parallel:
 - {N}, {N.}, etc.
 - parallel command {1} {2} {3} ::: 1 2 3 ::: 4 5 6 ::: 7 8 9
 - paralllel command ::: a b c :::+ 1 2 3 ::: d e f :::+ 4 5 6
 - Grouping currently only prints stdout/stderr of processes that are finished, instead
   of printing while they are running.
 - SSH support
*/

/// The `JobOutput` structure is utilized when grouping is enabled to transmit a command's
/// associated job ID with it's stdout and stderr buffers back to the main thread to be
/// queued for printing in the order that the inputs are supplied.
struct JobOutput {
    id:     usize,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn main() {
    // Obtain a handle to standard error's buffer so we can write directly to it.
    let stderr = io::stderr();

    // Set the default arguments for the application.
    let mut args = Args {
        ncores: num_cpus::get(),
        grouped: true,
        uses_shell: true,
        arguments: Vec::new(),
        inputs: Vec::new()
    };

    // Let's collect all parameters that we need from the program's arguments.
    // If an error is returned, this will handle that error as efficiently as possible.
    if let Err(why) = args.parse() {
        // Always lock an output buffer before using it.
        let mut stderr = stderr.lock();
        let _ = stderr.write(b"parallel: parsing error: ");
        match why {
            ParseErr::JobsNaN(value) => {
                let _ = write!(&mut stderr, "jobs parameter, '{}', is not a number.\n", value);
            },
            ParseErr::JobsNoValue => {
                let _ = stderr.write(b"no jobs parameter was defined.\n");
            },
            ParseErr::InvalidArgument(argument) => {
                let _ = write!(&mut stderr, "invalid argument: {}\n", argument);
            }
        };
        exit(1);
    }

    // If no command argument was given, then the inputs are actually commands themselves.
    let inputs_are_commands = args.arguments.is_empty();

    // It will be useful to know the number of inputs, to know when to quit.
    let num_inputs = args.inputs.len();

    // Keeps track of the current step in the input queue.
    // All threads will share this counter without stepping on each other's toes.
    let shared_counter = Arc::new(AtomicUsize::new(0));

    // We will share the same list of inputs with each thread.
    let shared_input = Arc::new(args.inputs);

    // If grouping is enabled, stdout and stderr will be buffered.
    let (output_tx, input_rx) = channel::<JobOutput>();

    // First we will create as many threads as `ncores` specifies.
    // The `threads` vector will contain the thread handles needed to
    // know when to quit the program.
    let mut threads: Vec<JoinHandle<()>> = Vec::with_capacity(args.ncores);

    // The `slot` variable is required by the {%} token.
    for slot in 1..args.ncores+1 {
        // The base command template that each thread will use.
        let arguments = args.arguments.clone();
        // Allow the thread to gain access to the list of inputs.
        let input = shared_input.clone();
        // Allow the thread to access the input counter
        let counter = shared_counter.clone();
        // Allow the thread to know when it's time to stop.
        let num_inputs = num_inputs;
        // If grouped is set to true, stdout/stderr buffers will be collected.
        let grouped = args.grouped;
        // If `uses_shell` is set to true, commands will be executed in the platform's shell.
        let uses_shell = args.uses_shell;
        // Each thread will receive it's own sender for sending stderr/stdout buffers.
        let output_tx = output_tx.clone();

        // The actual thread where the work will happen on incoming data.
        let handle: JoinHandle<()> = thread::spawn(move || {
            // Grab a handle to standard error for this thread.
            let stderr = io::stderr();

            // The {%} token requires to know the thread's ID.
            let slot = slot.to_string();
            // Stores the value for the {#^} token.
            let job_total = num_inputs.to_string();

            // Starts the thread's main loop. which will only break when all inputs are processed.
            loop {
                // Obtain the Nth item and it's job ID from the list of inputs.
                let (input_var, job_id) = {
                    // Atomically increment the counter
                    let counter = counter.fetch_add(1, Ordering::SeqCst);
                    // Check to see if all inputs have already been processed
                    if counter >= num_inputs { break } else { (&input[counter], (counter + 1)) }
                };
                if inputs_are_commands {
                    if grouped {
                        // Executes each input as if it were a command and returns a
                        // `Command::Output`, if the command executes successfully.
                        match command::get_command_output(input_var, uses_shell) {
                            Ok(ref output) => {
                                // Assign the `job_id` with the command's stdout and stderr
                                // buffers and transmit them back to the main thread.
                                output_tx.send(JobOutput{
                                    id:     job_id,
                                    stdout: output.stdout.clone(),
                                    stderr: output.stderr.clone(),
                                }).unwrap();
                            },
                            // The command has, sadly, failed. This will tell the user why.
                            Err(why) => {
                                let mut stderr = stderr.lock();
                                let _ = write!(&mut stderr, "parallel: command error: {}: {}\n",
                                    input_var, why);
                            }
                        }
                    } else {
                        // With no need to group the outputs, we only need to know the status
                        // of the command's execution. The standard output and standard error
                        // will automatically be inherited.
                        if let Err(why) = command::get_command_status(input_var, uses_shell) {
                            let mut stderr = stderr.lock();
                            let _ = stderr.write(b"parallel: command error:");
                            let _ = write!(&mut stderr, "{}: {}\n", input_var, why);
                        }
                    }
                } else {
                    // Build a command by merging the command template with the input,
                    // and then execute that command.
                    match command::exec(input_var, &arguments, &slot,
                        &job_id.to_string(), &job_total, grouped, uses_shell)
                    {
                        // If grouping is enabled, then we have an output to process.
                        Ok(Some(ref output)) if grouped => {
                            output_tx.send(JobOutput{
                                id: job_id,
                                stdout: output.stdout.clone(),
                                stderr: output.stderr.clone(),
                            }).unwrap();
                        },
                        // If grouping was not enabled, nothing was returned.
                        Ok(_) => (),
                        // I've an error handler already created for this error type.
                        Err(cmd_err) => {
                            let mut stderr = stderr.lock();
                            cmd_err.handle(&mut stderr);
                        }
                    }
                }
            }
        });

        // After the thread has been created, add the important pieces needed by the
        // main thread to the `threads` vector.
        threads.push(handle);
    }

    if args.grouped {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        let mut stderr = stderr.lock();

        // Job ID's start counting from `1`. This counter will keep track of what ID
        // we need to print next.
        let mut counter = 1;

        // If the job we receive is ahead of the current counter, we will queue it for
        // later printing in this `buffer` variable.
        let mut buffer = Vec::new();

        // Store a list of indexes we need to drop from `buffer` after a match has been found.
        let mut drop = Vec::with_capacity(args.ncores);

        // The loop will only quit once all inputs have been received. I guarantee it.
        while counter != num_inputs + 1 {
            // Block and wait until a new buffer is received.
            let output = input_rx.recv().unwrap();

            // If the buffer ID is the next in line, print it, else add it to the buffer.
            if output.id == counter {
                let _ = stdout.write(&output.stdout);
                let _ = stderr.write(&output.stderr);
                counter += 1;
            } else {
                buffer.push(output);
            }

            // Check to see if there are any stored buffers that can now be printed.
            // Items in the buffer will be removed after they are used.
            'outer: loop {
                // Keep track of if any changes have been made in this iteration.
                let mut changed = false;

                // Loop through the list of buffers and print buffers with the next ID in line.
                // If a match was found, changed will be set to true and the job added to the
                // drop list. If no change was found, the outer loop will quit.
                for (id, output) in buffer.iter().enumerate() {
                    if output.id == counter {
                        let _ = stdout.write(&output.stdout);
                        let _ = stderr.write(&output.stderr);
                        counter += 1;
                        changed = true;
                        drop.push(id);
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
