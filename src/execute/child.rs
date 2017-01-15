use arguments::QUIET_MODE;
use std::process::Child;
use std::sync::mpsc::Sender;
use std::time::Duration;
use wait_timeout::ChildExt;
use time::{get_time, Timespec};
use super::signals;
use super::pipe::disk::output as pipe_output;
use super::pipe::disk::State;

/// Receives a `Child` and handles the child according. If a `timeout` is specified then the process will be killed
/// if it exceeds the `timeout` value. Job stats are also gathered in case the `--joblog` parameter was supplied.
pub fn handle_child(mut child: Child, output: &Sender<State>, flags: u16, job_id: usize, input: String,
    has_timeout: bool, timeout: Duration, base: &str, buffer: &mut [u8]) -> (Timespec, Timespec, i32, i32)
{
    let start_time = get_time();
    if has_timeout && child.wait_timeout(timeout).unwrap().is_none() {
        let _ = child.kill();
        pipe_output(&mut child, job_id, input, output, flags & QUIET_MODE != 0, base, buffer);
        (start_time, get_time(), -1, 15)
    } else {
        pipe_output(&mut child, job_id, input, output, flags & QUIET_MODE != 0, base, buffer);
        match child.wait() {
            Ok(status) => match status.code() {
                Some(exit) => (start_time, get_time(), exit, 0),
                None       => (start_time, get_time(), -1, signals::get(status))
            },
            Err(_) => (start_time, get_time(), -1, 0),
        }
    }
}
