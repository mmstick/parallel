// #![deny(dead_code)]
// #![deny(unused_imports)]
#![allow(unknown_lints)]
#![feature(loop_break_value)]
#![feature(alloc_system)]
extern crate alloc_system;
extern crate arrayvec;
extern crate itoa;
extern crate num_cpus;
extern crate permutate;
extern crate smallvec;
extern crate sys_info;
extern crate time;
extern crate wait_timeout;

mod arguments;
mod disk_buffer;
mod execute;
mod filepaths;
mod init;
mod input_iterator;
mod misc;
mod tokenizer;
mod shell;
mod verbose;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::mem;
use std::process::exit;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;

use arguments::Args;
use execute::pipe::disk::State;
use input_iterator::{InputIterator, InputsLock};
use tokenizer::{Token, TokenErr, tokenize};

/// Coercing the `command` `String` into a `&'static str` is required to share it among all threads.
/// The command string needs to be available in memory for the entirety of the application, so this
/// is achievable by leaking the memory so it attains a `'static` lifetime.
unsafe fn leak_command(comm: String) -> &'static str {
    let static_comm = mem::transmute(&comm as &str);
    mem::forget(comm);
    static_comm
}

unsafe fn static_arg(args: &[Token]) -> &'static [Token] {
    mem::transmute(args)
}

fn main() {
    // Obtain a handle to standard error's buffer so we can write directly to it.
    let stdout = io::stdout();
    let stderr = io::stderr();

    // Cleanup pre-existing files from the filesystem before continuing.
    let (unprocessed_path, processed_path, errors_path) = init::cleanup(&mut stderr.lock());

    // Parse arguments and collect flags and statistics.
    let mut args = Args::new();
    let mut comm = String::with_capacity(128);
    let raw_arguments = env::args().collect::<Vec<String>>();
    args.ninputs = init::parse(&mut args, &mut comm, &raw_arguments, &unprocessed_path);

    // Initialize the `InputIterator` structure, which iterates through all inputs.
    let inputs = InputIterator::new(&unprocessed_path, args.ninputs)
        .expect("unable to initialize the InputIterator structure");

    // Coerce the `comm` `String` into a `&'static str` so that it may be shared by all threads.
    // This is safe because the original `comm` may no longer be modified due to shadowing rules.
    // It is also safe because `comm` lives to the end of the program.
    let comm = unsafe { leak_command(comm) };

    // Attempt to tokenize the command argument into simple primitive placeholders.
    if let Err(error) = tokenize(&mut args.arguments, comm, &unprocessed_path, args.ninputs) {
        let mut stderr = stderr.lock();
        match error {
            TokenErr::File(why) => {
                let _ = write!(stderr, "unable to obtain Nth input: {}\n", why);
            },
            TokenErr::OutOfBounds => {
                let _ = write!(stderr, "input token out of bounds\n");
            }
        }
        exit(1)
    }

    let arguments = unsafe { static_arg(&args.arguments) };

    if args.flags & arguments::DRY_RUN != 0 {
        execute::dry_run(args.flags, inputs, arguments);
    } else {
        if shell::dash_exists() { args.flags |= arguments::DASH_EXISTS; }
        if shell::required(shell::Kind::Tokens(arguments)) { args.flags |= arguments::SHELL_ENABLED; }

        let shared_input = Arc::new(Mutex::new(inputs));

        // If grouping is enabled, stdout and stderr will be buffered.
        let (output_tx, input_rx) = channel::<State>();

        // Will contain handles to the upcoming threads to know when the threads are finished.
        let mut threads = Vec::with_capacity(args.ncores);

        if args.flags & arguments::VERBOSE_MODE != 0 {
            verbose::total_inputs(&stdout, args.ncores, args.ninputs);
        }

        // The `slot` variable is required by the {%} token.
        if args.flags & arguments::INPUTS_ARE_COMMANDS != 0 {
            if shell::dash_exists() { args.flags |= arguments::DASH_EXISTS; }

            for _ in 0..args.ncores {
                let flags = args.flags;

                let mut exec = execute::ExecInputs {
                    num_inputs: args.ninputs,
                    timeout:    args.timeout,
                    output_tx:  output_tx.clone(),
                    inputs:     InputsLock {
                        inputs:    shared_input.clone(),
                        memory:    args.memory,
                        delay:     args.delay,
                        has_delay: args.delay != Duration::from_millis(0),
                        completed: false,
                        flags:     flags,
                    }
                };

                let handle: JoinHandle<()> = thread::spawn(move || {
                    exec.run(flags);
                });

                // Add the thread handle to the `threads` vector to know when to quit the program.
                threads.push(handle);
            }
        } else {
            shell::set_flags(&mut args.flags, arguments);

            for slot in 1..args.ncores+1 {
                let timeout = args.timeout;
                let num_inputs = args.ninputs;
                let output_tx = output_tx.clone();
                let flags = args.flags;

                let inputs = InputsLock {
                    inputs:    shared_input.clone(),
                    memory:    args.memory,
                    delay:     args.delay,
                    has_delay: args.delay != Duration::from_millis(0),
                    completed: false,
                    flags:     flags,
                };

                // The command will be built from the arguments, and inputs will be transferred to the command.
                let handle: JoinHandle<()> = thread::spawn(move || {
                    let mut exec = execute::ExecCommands {
                        slot:       slot,
                        num_inputs: num_inputs,
                        flags:      flags,
                        timeout:    timeout,
                        inputs:     inputs,
                        output_tx:  output_tx,
                        arguments:  arguments,
                    };
                    exec.run();
                });

                // Add the thread handle to the `threads` vector to know when to quit the program.
                threads.push(handle);
            }
        }

        /// Prints messages from executed commands in the correct order.
        execute::receive_messages(input_rx, args, &processed_path, &errors_path);
        for thread in threads { thread.join().unwrap(); }

        // If errors have occurred, re-print these errors at the end.
        if let Ok(file) = File::open(errors_path) {
            if file.metadata().ok().map_or(0, |metadata| metadata.len()) > 0 {
                let stderr = &mut stderr.lock();
                let _ = stderr.write(b"parallel: encountered errors during processing:\n");
                for line in BufReader::new(file).lines() {
                    if let Ok(line) = line {
                        let _ = stderr.write(line.as_bytes());
                        let _ = stderr.write(b"\n");
                    }
                }
                exit(1);
            }
        }
    }
}
