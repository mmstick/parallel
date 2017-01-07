pub fn basic(command: &str) -> String {
    let mut output = String::with_capacity(command.len() << 1);
    for character in command.chars() {
        if character == '\\' { output.push(character); }
        output.push(character);
    }
    output
}
