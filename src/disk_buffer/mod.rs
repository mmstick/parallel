use std::path::{Path, PathBuf};
use std::io::{Error, Read};

/// Controls the size of the buffers for reading/writing to files.
pub const BUFFER_SIZE: usize = 8 * 1024; // 8K seems to be the best buffer size.

pub trait DiskBufferTrait {
    /// Clears the buffered memory and resets the capacity.
    fn clear(&mut self);

    /// Obtain a slice of only the useful information.
    fn get_ref(&self) -> &[u8];

    /// Returns true if the buffer does not contain any contents.
    fn is_empty(&self) -> bool;
}

/// A `DiskBufferReader` contains the `buffer` method.
pub struct DiskBufferReader<IO: Read> {
    pub data:     [u8; BUFFER_SIZE],
    pub capacity: usize,
    pub file:     IO,
    pub path:     PathBuf,
}

impl<IO: Read> DiskBufferTrait for DiskBufferReader<IO> {
    fn clear(&mut self) { self.capacity = 0; }
    fn get_ref(&self) -> &[u8] { &self.data[0..self.capacity] }
    fn is_empty(&self) -> bool { self.capacity == 0 }
}

impl<IO: Read> DiskBufferReader<IO> {
    pub fn new<P: AsRef<Path>>(path: P, file: IO) -> DiskBufferReader<IO> {
        DiskBufferReader {
            data:     [b'\0'; BUFFER_SIZE],
            capacity: 0,
            file:     file,
            path:     path.as_ref().to_owned(),
        }
    }

    /// Reads the next set of bytes from the disk and stores them into memory.
    /// Takes an input argument that will optionally shift unused bytes at the end to the left,
    /// and then buffer into the adjacent bytes.
    pub fn buffer(&mut self, bytes_used: usize) -> Result<(), Error> {
        if bytes_used == 0 {
            self.capacity = self.file.read(&mut self.data)?;
        } else {
            let bytes_unused = self.capacity - bytes_used;
            for (left, right) in (0..bytes_unused).zip(bytes_used + 1..bytes_used + bytes_unused) {
                self.data[left] = self.data[right];
            }
            self.capacity = self.file.read(&mut self.data[bytes_unused-1..])? + bytes_unused - 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_disk_buffer_reader_simple() {
        let file = include_bytes!("../../tests/buffer.dat");
        let mut disk_buffer_reader = DiskBufferReader::new(Path::new("tests/buffer.dat"),
            File::open("tests/buffer.dat").expect("unable to open test data"));
        let _ = disk_buffer_reader.buffer(0);
        assert_eq!(&file[0..BUFFER_SIZE], &disk_buffer_reader.data[..]);
        let _ = disk_buffer_reader.buffer(0);
        assert_eq!(&file[BUFFER_SIZE..BUFFER_SIZE*2], &disk_buffer_reader.data[..]);
        let _ = disk_buffer_reader.buffer(0);
        assert_eq!(&file[BUFFER_SIZE*2..], &disk_buffer_reader.data[..2989]);
    }

    #[test]
    fn test_disk_buffer_reader_byte_shifting() {
        let file = include_bytes!("../../tests/buffer.dat");
        let mut disk_buffer_reader = DiskBufferReader::new(Path::new("tests/buffer.dat"),
            File::open("tests/buffer.dat").expect("unable to open test data"));
        let _ = disk_buffer_reader.buffer(0);
        assert_eq!(&file[0..BUFFER_SIZE], &disk_buffer_reader.data[..]);
        let _ = disk_buffer_reader.buffer(BUFFER_SIZE/2);
        assert_eq!(&file[BUFFER_SIZE/2+1..BUFFER_SIZE/2+1+BUFFER_SIZE], &disk_buffer_reader.data[0..BUFFER_SIZE]);
    }
}
