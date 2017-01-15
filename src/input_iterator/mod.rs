mod lock;
mod iterator;

pub use self::lock::InputsLock;
pub use self::iterator::{InputIterator, ETA};

use std::io;
use std::path::PathBuf;

/// The `InputIterator` may possibly encounter an error with reading from the unprocessed file.
#[derive(Debug)]
pub enum InputIteratorErr {
    FileRead(PathBuf, io::Error),
}
