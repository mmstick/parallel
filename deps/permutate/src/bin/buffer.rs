#[cfg(not(unix))]
pub mod platform {
    pub const BUFFER_SIZE: usize = 16 * 1024; // Windows only supports 16K buffers.
}

#[cfg(unix)]
pub mod platform {
    pub const BUFFER_SIZE: usize = 64 * 1024; // 4.75% performance boost over 16K buffers
}

use std::io::{StdoutLock, Write};
use self::platform::BUFFER_SIZE;

/// Manual buffer implementation for buffering standard output.
pub struct StdoutBuffer {
    pub data:     [u8; BUFFER_SIZE],
    pub capacity: usize,
}

impl StdoutBuffer {
    /// Create a new `Buffer` initialized to be empty.
    pub fn new() -> StdoutBuffer {
        StdoutBuffer { data: [0u8; BUFFER_SIZE], capacity: 0 }
    }

    /// Write the buffer's contents to stdout and clear itself.
    pub fn write_and_clear(&mut self, stdout: &mut StdoutLock) {
        let _ = stdout.write_all(&self.data[..]);
        self.data = [0u8; BUFFER_SIZE];
        self.capacity = 0;
    }

    /// Write a byte slice to the buffer and mark the new size.
    pub fn write(&mut self, data: &[u8]) {
        let cap = data.len();
        self.data[self.capacity..self.capacity + cap].clone_from_slice(data);
        self.capacity += cap;
    }

    /// Append an individual byte to the buffer, typically a space or newline.
    pub fn push(&mut self, data: u8) {
        self.data[self.capacity] = data;
        self.capacity += 1;
    }
}
