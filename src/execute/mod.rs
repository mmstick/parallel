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
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

pub use self::dry::dry_run;
pub use self::exec_commands::ExecCommands;
pub use self::exec_inputs::ExecInputs;
pub use self::receive::receive_messages;

pub struct InputsLock {
    pub inputs:    Arc<Mutex<InputIterator>>,
    pub memory:    u64,
    pub delay:     Duration,
    pub has_delay: bool,
    pub completed: bool,
    pub flags:     u16
}

impl InputsLock {
    pub fn try_next(&mut self, input: &mut String) -> Option<(usize, ETA)> {
        let mut inputs = self.inputs.lock().unwrap();
        let job_id = inputs.curr_argument;
        let eta = inputs.eta();
        if self.flags & arguments::ETA != 0 {
            if self.completed {
                inputs.completed += 1;
            } else {
                self.completed = true;
            }
            let stderr = io::stderr();
            let mut stderr = &mut stderr.lock();
            let message = format!("ETA: {}s Left: {} AVG: {:.2}s Completed: {}\n", eta.time / 1_000_000_000,
                eta.left, eta.average as f64 / 1_000_000_000f64, inputs.completed);
            let _ = stderr.write(message.as_bytes());
        }

        if self.has_delay { thread::sleep(self.delay); }

        if self.memory > 0 {
            if let Ok(mut mem_available) = sys_info::mem_info().map(|mem_info| mem_info.avail * 1000) {
                while mem_available < self.memory {
                    thread::sleep(Duration::from_millis(100));
                    if let Ok(mem_info) = sys_info::mem_info() { mem_available = mem_info.avail * 1000; }
                }
            }
        }

        match inputs.next_value(input) {
            None            => None,
            Some(Ok(())) => Some((job_id, eta)),
            Some(Err(why))  => {
                let stderr = io::stderr();
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
}
