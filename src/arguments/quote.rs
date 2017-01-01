pub fn basic(command: &str) -> String {
    let mut output = String::with_capacity(command.len() << 1);
    for character in command.chars() {
        if character == '\\' { output.push(character); }
        output.push(character);
    }
    output
}

pub fn shell(command: &str) -> String {
    let mut output = String::with_capacity(command.len() << 1);
    {
        let mut char_iter = command.chars();

        // Do not escape the command
        while let Some(character) = char_iter.next() {
            output.push(character);
            if character == ' ' { break }
        }

        // Escape all following arguments
        for character in char_iter {
            match character {
                '$' | ' ' | '\\' | '>' | '<' | '^' | '&' | '#' | '!' | '*' | '\'' | '\"' | '`' | '~' | '{' | '}' | '[' |
                ']' | '(' | ')' | ';' | '|' | '?' => output.push('\\'),
                _ => ()
            }
        }
    }

    output
}
