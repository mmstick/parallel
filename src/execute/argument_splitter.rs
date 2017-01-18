const DOUBLE: u8 = 1;
const SINGLE: u8 = 2;
const BACK:   u8 = 4;

/// An efficient `Iterator` structure for splitting arguments
pub struct ArgumentSplitter<'a> {
    buffer:       Vec<u8>,
    data:         &'a str,
    read:         usize,
    flags:        u8,
}

impl<'a> ArgumentSplitter<'a> {
    pub fn new(data: &'a str) -> ArgumentSplitter<'a> {
        ArgumentSplitter {
            buffer:       Vec::with_capacity(32),
            data:         data,
            read:         0,
            flags:        0,
        }
    }
}

impl<'a> Iterator for ArgumentSplitter<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Vec<u8>> {
        for character in self.data.bytes().skip(self.read) {
            self.read += 1;
            match character {
                _ if self.flags & BACK != 0 => {
                    self.buffer.push(character);
                    self.flags ^= BACK;
                },
                b'"'  if self.flags & SINGLE == 0 => self.flags ^= DOUBLE,
                b'\'' if self.flags & DOUBLE == 0 => self.flags ^= SINGLE,
                b' '  if !self.buffer.is_empty() & (self.flags & (SINGLE + DOUBLE) == 0) => break,
                b'\\' if (self.flags & (SINGLE + DOUBLE) == 0) => self.flags ^= BACK,
                _ => self.buffer.push(character)
            }
        }

        if self.buffer.is_empty() {
            None
        } else {
            let mut output = self.buffer.clone();
            output.shrink_to_fit();
            self.buffer.clear();
            Some(output)
        }
    }
}

#[test]
fn test_split_args() {
    use std::str;

    let argument = ArgumentSplitter::new("ffmpeg -i \"file with spaces\" \"output with spaces\"");
    let expected = vec!["ffmpeg", "-i", "file with spaces", "output with spaces"];
    let argument = argument.collect::<Vec<Vec<u8>>>();
    let argument = argument.iter().map(|x| str::from_utf8(x).unwrap()).collect::<Vec<&str>>();
    assert_eq!(argument, expected);

    let argument = ArgumentSplitter::new("one\\ two\\\\ three");
    let expected = vec!["one two\\", "three"];
    let argument = argument.collect::<Vec<Vec<u8>>>();
    let argument = argument.iter().map(|x| str::from_utf8(x).unwrap()).collect::<Vec<&str>>();
    assert_eq!(argument, expected);
}
