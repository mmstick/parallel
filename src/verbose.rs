use std::io::{Stdout, Write};
use itoa;

pub fn total_inputs(stdout: &Stdout, threads: usize, inputs: usize) {
    let mut stdout = stdout.lock();
    let _ = stdout.write(b"parallel: processing ");
    let _ = itoa::write(&mut stdout, inputs);
    let _ = stdout.write(b" inputs on ");
    let _ = itoa::write(&mut stdout, threads);
    let _ = stdout.write(b" threads\n");
}

pub fn processing_task(stdout: &Stdout, job: usize, total: usize, input: &str) {
    let mut stdout = stdout.lock();
    let _ = stdout.write(b"parallel: processing task #");
    let _ = itoa::write(&mut stdout, job);
    let _ = stdout.write(b" of ");
    let _ = itoa::write(&mut stdout, total);
    let _ = stdout.write(b": '");
    let _ = stdout.write(input.as_bytes());
    let _ = stdout.write(b"'\n");
}

pub fn task_complete(stdout: &Stdout, job: usize, total: usize, input: &str) {
    let mut stdout = stdout.lock();
    let _ = stdout.write(b"parallel:  completed task #");
    let _ = itoa::write(&mut stdout, job);
    let _ = stdout.write(b" of ");
    let _ = itoa::write(&mut stdout, total);
    let _ = stdout.write(b": '");
    let _ = stdout.write(input.as_bytes());
    let _ = stdout.write(b"'\n");
}
