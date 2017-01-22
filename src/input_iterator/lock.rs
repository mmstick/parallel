use arguments;
use super::{InputIterator, InputIteratorErr};
use sys_info;

use std::thread;
use std::time::Duration;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};

pub struct InputsLock<IO: Read> {
    pub inputs:    Arc<Mutex<InputIterator<IO>>>,
    pub memory:    u64,
    pub delay:     Duration,
    pub has_delay: bool,
    pub completed: bool,
    pub flags:     u16
}

impl<IO: Read> InputsLock<IO> {
    /// Attempts to obtain the next input in the queue, returning `None` when it is finished.
    /// It works the same as the `Iterator` trait's `next()` method, only re-using the same input buffer.
    pub fn try_next(&mut self, input: &mut String) -> Option<(usize)> {
        let mut inputs = self.inputs.lock().unwrap();
        let job_id = inputs.curr_argument;
        if self.flags & arguments::ETA != 0 {
            if self.completed {
                inputs.completed += 1;
            } else {
                self.completed = true;
            }
            inputs.eta().write_to_stderr(inputs.completed);
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
            Some(Ok(()))    => Some(job_id),
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
