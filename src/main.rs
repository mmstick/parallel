extern crate num_cpus;
mod command;
mod tokenizer;
mod parser;

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
    let stderr = io::stderr();
    let mut ncores = num_cpus::get();
    let mut command = String::new();
    let mut arg_tokens = Vec::new();
    let mut inputs = Vec::new();

    // Let's collect all parameters that we need from the program's arguments.
    // If an error is returned, this will handle that error as efficiently as possible.
    if let Err(why) = parse_arguments(&mut ncores, &mut command, &mut arg_tokens, &mut inputs) {
        let mut stderr = stderr.lock();
        let _ = stderr.write(b"parallel: parsing error: ");
        match why {
            ParseErr::JobsNaN(value) => {
                let _ = stderr.write(b"jobs parameter, '");
                let _ = stderr.write(value.as_bytes());
                let _ = stderr.write(b"', is not a number.\n");
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

    // Stores the next input to be processed
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
        // The arguments for the command.
        let argument_tokens = arg_tokens.clone();
        // Allow the thread to gain access to the list of inputs.
        let input = shared_input.clone();
        // Allow the thread to access the current command counter
        let counter = shared_counter.clone();
        // Allow the thread to know when it's time to stop.
        let num_inputs = num_inputs;

        // The actual thread where the work will happen on incoming data.
        let handle: JoinHandle<()> = thread::spawn(move || {
            let slot_number = slot;
            let stderr = io::stderr();
            loop {
                // Obtain the Nth item and it's job ID from the list of inputs.
                let (input_var, job_id) = {
                    // Atomically increment the counter
                    let old_counter = counter.fetch_add(1, Ordering::SeqCst);
                    if old_counter >= num_inputs {
                        break
                    } else {
                        let input_var = &input[old_counter];
                        let job_id = old_counter + 1;
                        (input_var, job_id)
                    }
                };

                if input_is_command {
                    // The inputs are actually the commands.
                    let mut iterator = input_var.split_whitespace();
                    let actual_command = iterator.next().unwrap();
                    let args = iterator.collect::<Vec<&str>>();
                    if let Err(_) = Command::new(actual_command).args(&args).status() {
                        let mut stderr = stderr.lock();
                        let _ = stderr.write(b"parallel: command error: ");
                        let _ = stderr.write(input_var.as_bytes());
                        let _ = stderr.write(b"\n");
                    }
                } else {
                    // Build a command by merging the command template with the input,
                    // and then execute that command.
                    let (slot, job) = (slot_number.to_string(), job_id.to_string());
                    if let Err(cmd_err) = command::exec(input_var, &command, &argument_tokens,
                        &slot, &job)
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

    for thread in threads.into_iter() { thread.join().unwrap(); }
}
