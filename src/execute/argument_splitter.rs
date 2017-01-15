const DOUBLE: u8 = 1;
const SINGLE: u8 = 2;
const BACK:   u8 = 4;

/// An efficient `Iterator` structure for splitting arguments
pub struct ArgumentSplitter<'a> {
    buffer:       String,
    data:         &'a str,
    read:         usize,
    flags:        u8,
}

impl<'a> ArgumentSplitter<'a> {
    pub fn new(data: &'a str) -> ArgumentSplitter<'a> {
        ArgumentSplitter {
            buffer:       String::with_capacity(32),
            data:         data,
            read:         0,
            flags:        0,
        }
    }
}

impl<'a> Iterator for ArgumentSplitter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        for character in self.data.chars().skip(self.read) {
            self.read += 1;
            match character {
                _ if self.flags & BACK != 0 => {
                    self.buffer.push(character);
                    self.flags ^= BACK;
                },
                '"'  if self.flags & SINGLE == 0 => self.flags ^= DOUBLE,
                '\'' if self.flags & DOUBLE == 0 => self.flags ^= SINGLE,
                ' '  if !self.buffer.is_empty() & (self.flags & (SINGLE + DOUBLE) == 0) => break,
                '\\' if (self.flags & (SINGLE + DOUBLE) == 0) => self.flags ^= BACK,
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
    let argument = ArgumentSplitter::new("ffmpeg -i \"file with spaces\" \"output with spaces\"");
    let expected = vec!["ffmpeg", "-i", "file with spaces", "output with spaces"];
    assert_eq!(argument.collect::<Vec<String>>(), expected);

    let argument = ArgumentSplitter::new("one\\ two\\\\ three");
    let expected = vec!["one two\\", "three"];
    assert_eq!(argument.collect::<Vec<String>>(), expected);
}
