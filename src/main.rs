extern crate num_cpus;
mod command;   // Contains the functionality for building and processing external commands.
mod tokenizer; // Takes the command template that is provided and reduces it to digestible tokens.
mod parser;    // Collects the input arguments given to the program.

use std::io::{self, Write};
use std::process::{Command, exit};
use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use parser::{ParseErr, parse_arguments};

/* TODO: Functionality can be increased to accept the following syntaxes from GNU Parallel:
 - Stdin support is currently missing.
 - {N}, {N.}, etc.
 - parallel command {1} {2} {3} ::: 1 2 3 ::: 4 5 6 ::: 7 8 9
 - paralllel command ::: a b c :::+ 1 2 3 ::: d e f :::+ 4 5 6
*/

fn main() {
    // Obtain a handle to standard error's buffer so we can write directly to it.
    let stderr = io::stderr();

    // The `num_cpus` crate allows conveniently obtaining the number of CPU cores in the system.
    // This number will be used to determine how many threads to run in parallel.
    let mut ncores = num_cpus::get();

    // Initialize mutable vectors to store data that will be collected from input arguments.
    let mut command = String::new();
    let mut argument_tokens = Vec::new();
    let mut inputs = Vec::new();

    // Let's collect all parameters that we need from the program's arguments.
    // If an error is returned, this will handle that error as efficiently as possible.
    if let Err(why) = parse_arguments(&mut ncores, &mut command, &mut argument_tokens, &mut inputs) {
        // Always lock an output buffer before using it.
        let mut stderr = stderr.lock();
        let _ = stderr.write(b"parallel: parsing error: ");
        match why {
            ParseErr::JobsNaN(value) => {
                let _ = write!(&mut stderr, "jobs parameter, '{}', is not a number.\n", value);
            },
            _ => {
                let message: &[u8] = match why {
                    ParseErr::InputVarsNotDefined => b"input variables were not defined.\n",
                    ParseErr::JobsNoValue         => b"no jobs parameter was defined.\n",
                    _ => unreachable!()
                };
                let _ = stderr.write(message);
            }
        };
        exit(1);
    }

    // If no command was given, then the inputs are actually commands themselves.
    let input_is_command = command.is_empty();

    // It will be useful to know the number of inputs, to know when to quit.
    let num_inputs = inputs.len();

    // Keeps track of the current step in the input queue.
    // All threads will share this counter without stepping on each other's toes.
    let shared_counter = Arc::new(AtomicUsize::new(0));

    // We will share the same list of inputs with each thread.
    let shared_input = Arc::new(inputs);

    // First we will create as many threads as `ncores` specifies.
    // The `threads` vector will contain the thread handles needed to
    // know when to quit the program.
    let mut threads: Vec<JoinHandle<()>> = Vec::with_capacity(ncores);
    for slot in 1..ncores+1 {
        // The command that each input variable will be sent to.
        let command = command.clone();
        // The base command template that each thread will use.
        let argument_tokens = argument_tokens.clone();
        // Allow the thread to gain access to the list of inputs.
        let input = shared_input.clone();
        // Allow the thread to access the input counter
        let counter = shared_counter.clone();
        // Allow the thread to know when it's time to stop.
        let num_inputs = num_inputs;

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
                    if counter >= num_inputs {
                        // If the counter is >= the total number of inputs, processing is finished.
                        break
                    } else {
                        // Obtain the Nth input as well as the job ID
                        (&input[counter], (counter + 1).to_string())
                    }
                };

                if input_is_command {
                    // The inputs are actually the commands.
                    let mut iterator = input_var.split_whitespace();
                    // There will always be at least one argument: the command.
                    let actual_command = iterator.next().unwrap();
                    // The rest of the fields are arguments, if there are any arguments.
                    let args = iterator.collect::<Vec<&str>>();
                    // Attempt to run the current input as a command.
                    if let Err(_) = Command::new(actual_command).args(&args).status() {
                        let mut stderr = stderr.lock();
                        let _ = write!(&mut stderr, "parallel: command error: {}\n", input_var);
                    }
                } else {
                    // Build a command by merging the command template with the input,
                    // and then execute that command.
                    if let Err(cmd_err) = command::exec(input_var, &command, &argument_tokens,
                        &slot, &job_id, &job_total)
                    {
                        let mut stderr = stderr.lock();
                        cmd_err.handle(&mut stderr);
                    }
                }
            }
        });

        // After the thread has been created, add the important pieces needed by the
        // main thread to the `threads` vector.
        threads.push(handle);
    }

    // Wait for each thread to complete before quitting the program.
    for thread in threads.into_iter() { thread.join().unwrap(); }
}
