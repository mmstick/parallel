#![deny(dead_code)]
#![deny(unused_imports)]
#![allow(unknown_lints)]
#![feature(loop_break_value)]
#![feature(alloc_system)]
extern crate alloc_system;
extern crate arrayvec;
extern crate num_cpus;
extern crate permutate;

mod arguments;
mod command;
mod disk_buffer;
mod filepaths;
mod init;
mod input_iterator;
mod threads;
mod tokenizer;
mod verbose;

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::mem;
use std::process::exit;
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;

use arguments::Args;
use threads::pipe::disk::State;
use tokenizer::{Token, TokenErr, tokenize};

/// Coercing the `command` `String` into a `&'static str` is required to share it among all threads.
/// The command string needs to be available in memory for the entirety of the application, so this
/// is achievable by leaking the memory so it attains a `'static` lifetime.
unsafe fn leak_command(comm: String) -> &'static str {
    let static_comm = mem::transmute(&comm as &str);
    mem::forget(comm);
    static_comm
}

/// Determines if a shell is required or not for execution
fn shell_required(arguments: &[Token]) -> bool {
    for token in arguments {
        if let Token::Argument(ref arg) = *token {
            if arg.contains(';') || arg.contains('&') || arg.contains('|') {
                return true
            }
        }
    }
    false
}

/// Returns `true` if the Dash shell was found within the `PATH` environment variable.
fn dash_exists() -> bool {
    if let Ok(path) = env::var("PATH") {
        for path in path.split(':') {
            if let Ok(directory) = fs::read_dir(path) {
                for entry in directory {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.file_name() == Some(OsStr::new("dash")) { return true; }
                    }
                }
            }
        }
    }
    false
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
    let inputs = init::parse(&mut args, &mut comm, &raw_arguments, &unprocessed_path);

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

    if args.flags & arguments::DRY_RUN != 0 {
        threads::execute::dry_run(args.flags, inputs, &args.arguments);
    } else {
        // Determines if a shell is required or not
        if shell_required(&args.arguments) {
            if dash_exists() {
                args.flags |= arguments::DASH_EXISTS + arguments::SHELL_ENABLED;
            } else {
                args.flags |= arguments::SHELL_ENABLED;
            }
        }

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
            for _ in 0..args.ncores {
                let num_inputs = args.ninputs;
                let inputs     = shared_input.clone();
                let flags      = args.flags;
                let output_tx  = output_tx.clone();

                // Each input will be treated as a command
                let handle: JoinHandle<()> = thread::spawn(move || {
                    threads::execute::inputs(num_inputs, flags, inputs, output_tx)
                });

                // Add the thread handle to the `threads` vector to know when to quit the program.
                threads.push(handle);
            }
        } else {
            for slot in 1..args.ncores+1 {
                let num_inputs = args.ninputs;
                let inputs     = shared_input.clone();
                let flags      = args.flags;
                let output_tx  = output_tx.clone();
                let arguments  = args.arguments.clone();

                // The command will be built from the arguments, and inputs will be transferred to the command.
                let handle: JoinHandle<()> = thread::spawn(move || {
                    threads::execute::command(slot, num_inputs, flags, &arguments, inputs, output_tx)
                });

                // Add the thread handle to the `threads` vector to know when to quit the program.
                threads.push(handle);
            }
        }

        threads::receive_messages(input_rx, args, &processed_path, &errors_path);
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
