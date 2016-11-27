#![feature(alloc_system)]
extern crate alloc_system;
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

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process::exit;
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;

use arguments::Args;
use threads::pipe::State;

fn main() {
    // Obtain a handle to standard error's buffer so we can write directly to it.
    let stdout = io::stdout();
    let stderr = io::stderr();

    // Cleanup pre-existing files from the filesystem before continuing.
    init::cleanup(&mut stderr.lock());

    // Parse arguments and collect flags and statistics.
    let mut args = Args::new();
    let inputs = init::parse(&mut args);
    let shared_input = Arc::new(Mutex::new(inputs));

    // If grouping is enabled, stdout and stderr will be buffered.
    let (output_tx, input_rx) = channel::<State>();

    // Will contain handles to the upcoming threads to know when the threads are finished.
    let mut threads = Vec::with_capacity(args.ncores);

    if args.flags.verbose {
        verbose::total_inputs(&stdout, args.ncores, args.ninputs);
    }

    // The `slot` variable is required by the {%} token.
    if args.flags.inputs_are_commands {
        for _ in 0..args.ncores {
            let num_inputs = args.ninputs;
            let inputs     = shared_input.clone();
            let flags      = args.flags.clone();
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
            let flags      = args.flags.clone();
            let output_tx  = output_tx.clone();
            let arguments = args.arguments.clone();

            // The command will be built from the arguments, and inputs will be transferred to the command.
            let handle: JoinHandle<()> = thread::spawn(move || {
                threads::execute::command(slot, num_inputs, flags, arguments, inputs, output_tx)
            });

            // Add the thread handle to the `threads` vector to know when to quit the program.
            threads.push(handle);
        }
    }

    threads::receive_messages(input_rx, args);
    for thread in threads { thread.join().unwrap(); }

    // If errors have occurred, re-print these errors at the end.
    if let Some(path) = filepaths::errors() {
        if let Ok(file) = File::open(path) {
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
