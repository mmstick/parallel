use std::ptr::swap;

pub fn reverse(string: &mut [u8], length: usize) {
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

pub fn itoa(string: &mut [u8], mut number: usize, base: usize) -> usize {
    if number == 0 {
        string[0] = b'0';
        return 1;
    }

    let mut index = 0;
    while number != 0 {
        let rem = (number % base) as u8;
        string[index] = if rem > 9 { (rem - 10) + b'a' } else { (rem + b'0')  };
        index += 1;
        number /= base;
    }

    reverse(string, index);
    index
}
