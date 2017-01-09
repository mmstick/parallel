use arguments;
use tokenizer::Token;
use std::env;
use std::fs;
use std::ffi::OsStr;

pub enum Kind<'a> {
    Tokens(&'a [Token]),
    Input(&'a str)
}

/// Determines if a shell is required or not for execution
pub fn required(kind: Kind) -> bool {
    match kind {
        Kind::Tokens(arguments) => {
            for token in arguments {
                if let Token::Argument(ref arg) = *token {
                    if arg.contains(';') || arg.contains('&') || arg.contains('|') {
                        return true
                    }
                }
            }
        },
        Kind::Input(arg) => if arg.contains(';') || arg.contains('&') || arg.contains('|') {
            return true
        }
    }
    false
}

/// Returns `true` if the Dash shell was found within the `PATH` environment variable.
pub fn dash_exists() -> bool {
    if let Ok(path) = env::var("PATH") {
        for path in path.split(':') {
            if let Ok(directory) = fs::read_dir(path) {
                for entry in directory {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.file_name() == Some(OsStr::new("dash")) { return true; }
                    }
                }
            }
        }
    }
    false
}

/// Sets the corresponding flags if a shell is required and if dash exists.
pub fn set_flags(flags: &mut u16, arguments: &[Token]) {
    if required(Kind::Tokens(arguments)) {
        println!("Required");
        if dash_exists() {
            *flags |= arguments::SHELL_ENABLED + arguments::DASH_EXISTS;
        } else {
            *flags |= arguments::SHELL_ENABLED;
        }
    }
}
