mod dry;
mod exec_commands;
mod exec_inputs;
pub mod pipe;
mod receive;

use arguments::{self, InputIteratorErr};
use command;
use super::input_iterator::{ETA, InputIterator};
use sys_info;

use std::thread;
use std::time::Duration;
use std::io::{Write, Stderr};
use std::sync::{Arc, Mutex};

pub use self::dry::dry_run;
pub use self::exec_commands::ExecCommands;
pub use self::exec_inputs::ExecInputs;
pub use self::receive::receive_messages;

// Attempts to obtain the next input argument along with it's job ID from the `InputIterator`.
// NOTE: Some reason this halves the wall time compared to making this a method of `InputIterator`.
fn attempt_next(inputs: &Arc<Mutex<InputIterator>>, stderr: &Stderr, has_delay: bool, delay: Duration,
    completed: &mut bool, memory: u64, flags: u16) -> Option<(String, usize, ETA)>
{
    let mut inputs = inputs.lock().unwrap();
    let job_id = inputs.curr_argument;
    let eta = inputs.eta();
    if flags & arguments::ETA != 0 {
        if *completed {
            inputs.completed += 1;
        } else {
            *completed = true;
        }
        println!("ETA: {}s Left: {} AVG: {:.2}s Completed: {}",
            eta.time / 1_000_000_000, eta.left, eta.average as f64 / 1_000_000_000f64,
            inputs.completed);
    }

    if has_delay { thread::sleep(delay); }

    if memory > 0 {
        if let Ok(mut mem_available) = sys_info::mem_info().map(|mem_info| mem_info.avail * 1000) {
            while mem_available < memory {
                thread::sleep(Duration::from_millis(100));
                if let Ok(mem_info) = sys_info::mem_info() { mem_available = mem_info.avail * 1000; }
            }
        }
    }

    match inputs.next() {
        None            => None,
        Some(Ok(input)) => Some((input, job_id, eta)),
        Some(Err(why))  => {
            let stderr = &mut stderr.lock();
            match why {
                InputIteratorErr::FileRead(path, why) => {
                    let _ = write!(stderr, "parallel: input file read error: {:?}: {}\n", path, why);
                },
            }
            None
        }
    }
}
