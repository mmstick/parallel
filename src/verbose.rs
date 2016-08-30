use std::io::{Stdout, Write};

pub fn total_inputs(stdout: &Stdout, threads: usize, inputs: usize) {
    let mut stdout = stdout.lock();
    let _ = stdout.write(b"parallel: processing ");
    let _ = stdout.write(inputs.to_string().as_bytes());
    let _ = stdout.write(b" inputs on ");
    let _ = stdout.write(threads.to_string().as_bytes());
    let _ = stdout.write(b" threads\n");
}

pub fn processing_task(stdout: &Stdout, job: &str, total: &str, input: &str) {
    let mut stdout = stdout.lock();
    let _ = stdout.write(b"parallel: processing task #");
    let _ = stdout.write(job.as_bytes());
    let _ = stdout.write(b" of ");
    let _ = stdout.write(total.as_bytes());
    let _ = stdout.write(b": '");
    let _ = stdout.write(input.as_bytes());
    let _ = stdout.write(b"'\n");
}

pub fn task_complete(stdout: &Stdout, job: &str, total: &str, input: &str) {
    let mut stdout = stdout.lock();
    let _ = stdout.write(b"parallel:  completed task #");
    let _ = stdout.write(job.as_bytes());
    let _ = stdout.write(b" of ");
    let _ = stdout.write(total.as_bytes());
    let _ = stdout.write(b": '");
    let _ = stdout.write(input.as_bytes());
    let _ = stdout.write(b"'\n");
}
