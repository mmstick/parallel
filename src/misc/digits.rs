/// A trait that adds the ability for numbers to find their digit count and to convert them to padded strings.
pub trait Digits {
    /// Counts the number of digits in a number. **Example:** {{0 = 0}, {1 = 1}, {10 = 2}, {100 = 3}}
    fn digits(&self) -> Self;
}

impl Digits for usize {
    fn digits(&self) -> usize {
        let mut digits = if *self == 1 || *self % 10 == 0 { 1 } else { 0 };
        let mut temp = 1;
        while temp < *self {
            digits += 1;
            temp = (temp << 3) + (temp << 1);
        }
        digits
    }
}
