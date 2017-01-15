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

// A lookup table to prevent the need for conditional branching
// The value of the remainder of each step will be used as the index
const LOOKUP: &'static [u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

macro_rules! impl_unsized_numtoa_for {
    ($t:ty) => {
        impl NumToA<$t> for $t {
            fn numtoa(mut self, base: $t, string: &mut [u8]) -> usize {
                if self == 0 {
                    string[0] = b'0';
                    return 1;
                }

                let mut index = 0;
                while self != 0 {
                    let rem = self % base;
                    string[index] = LOOKUP[rem as usize];
                    index += 1;
                    self /= base;
                }

                reverse(string, index);
                index
            }
        }
    }
}

macro_rules! impl_sized_numtoa_for {
    ($t:ty) => {
        impl NumToA<$t> for $t {
            fn numtoa(mut self, base: $t, string: &mut [u8]) -> usize {
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
                    let rem = self % base;
                    string[index] = LOOKUP[rem as usize];
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

    }
}

impl_sized_numtoa_for!(i8);
impl_sized_numtoa_for!(i16);
impl_sized_numtoa_for!(i32);
impl_sized_numtoa_for!(i64);
impl_sized_numtoa_for!(isize);
impl_unsized_numtoa_for!(u8);
impl_unsized_numtoa_for!(u16);
impl_unsized_numtoa_for!(u32);
impl_unsized_numtoa_for!(u64);
impl_unsized_numtoa_for!(usize);
