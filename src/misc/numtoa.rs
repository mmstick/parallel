use std::ptr::swap;

/// Because the integer to string conversion writes the representation in reverse, this will correct it.
fn reverse(string: &mut [u8], length: usize) {
    let mut start = 0isize;
    let mut end   = length as isize - 1;
    while start < end {
        unsafe {
            let x = string.as_mut_ptr().offset(start);
            let y = string.as_mut_ptr().offset(end);
            swap(x, y);
        }
        start += 1;
        end -= 1;
    }
}

/// Converts a number into a string representation, storing the conversion into a mutable byte slice.
pub trait NumToA<T> {
    /// Given a base for encoding and mutable byte slice, write the number into the byte slice and return the
    /// amount of bytes that were written.
    fn numtoa(self, base: T, string: &mut [u8]) -> usize;
}

impl NumToA<i32> for i32 {
    fn numtoa(mut self, base: i32, string: &mut [u8]) -> usize {
        let mut index = 0;
        let mut is_negative = false;

        if self < 0 {
            is_negative = true;
            self = self.abs();
        } else if self == 0 {
            string[0] = b'0';
            return 1;
        }

        while self != 0 {
            let rem = (self % base) as u8;
            string[index] = if rem > 9 { (rem - 10) + b'a' } else { (rem + b'0')  };
            index += 1;
            self /= base;
        }

        if is_negative {
            string[index] = b'-';
            index += 1;
        }

        reverse(string, index);
        index
    }
}

impl NumToA<usize> for usize {
    fn numtoa(mut self, base: usize, string: &mut [u8]) -> usize {
        if self == 0 {
            string[0] = b'0';
            return 1;
        }

        let mut index = 0;
        while self != 0 {
            let rem = (self % base) as u8;
            string[index] = if rem > 9 { (rem - 10) + b'a' } else { (rem + b'0')  };
            index += 1;
            self /= base;
        }

        reverse(string, index);
        index
    }
}


impl NumToA<u64> for u64 {
    fn numtoa(mut self, base: u64, string: &mut [u8]) -> usize {
        if self == 0 {
            string[0] = b'0';
            return 1;
        }

        let mut index = 0;
        while self != 0 {
            let rem = (self % base) as u8;
            string[index] = if rem > 9 { (rem - 10) + b'a' } else { (rem + b'0')  };
            index += 1;
            self /= base;
        }

        reverse(string, index);
        index
    }
}
