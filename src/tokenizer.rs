#[derive(Clone, PartialEq, Debug)]
/// A token is a placeholder for the operation to be performed on the input value.
pub enum Token {
    /// An argument is simply a collection of characters that are not placeholders.
    Argument(String),
    /// Takes the basename (file name) of the input with the extension removed.
    BaseAndExt,
    /// Takes the basename (file name) of the input with the directory path removed.
    Basename,
    /// Takes the directory path of the input with the basename removed.
    Dirname,
    /// Returns the job ID of the current input.
    Job,
    /// Returns the total number of jobs.
    JobTotal,
    /// Takes the input, unmodified.
    Placeholder,
    /// Removes the extension from the input.
    RemoveExtension,
    /// Returns the thread ID.
    Slot
}

/// Takes the command arguments as the input and reduces it into tokens,
/// which allows for easier management of string manipulation later on.
pub fn tokenize(tokens: &mut Vec<Token>, template: &str) {
    // When set to true, the characters following will be collected into `pattern`.
    let mut pattern_matching = false;
    // Mark the index where the pattern's first character begins.
    let mut pattern_start = 0;

    // Defines that an argument string is currently being matched
    let mut argument_matching = false;
    // Mark the index where the argument's first character begins.
    let mut argument_start = 0;

    for (id, character) in template.chars().enumerate() {
        match (character, pattern_matching) {
            // This condition initiates the pattern matching
            ('{', false) => {
                pattern_matching = true;
                pattern_start    = id;

                // If pattern matching has initialized while argument matching was happening,
                // this will append the argument to the token list and disable argument matching.
                if argument_matching {
                    argument_matching = false;
                    let argument      = template[argument_start..id].to_owned();
                    tokens.push(Token::Argument(argument));
                }
            },
            // This condition ends the pattern matching process
            ('}', true)  => {
                pattern_matching = false;
                if id == pattern_start+1 {
                    // This condition will be met when the pattern is "{}".
                    tokens.push(Token::Placeholder);
                } else {
                    // Supply the internal contents of the pattern to the token matcher.
                    match match_token(&template[pattern_start+1..id]) {
                        // If the token is a match, add the matched token.
                        Some(token) => tokens.push(token),
                        // If the token is not a match, add it as an argument.
                        None => {
                            tokens.push(Token::Argument(template[pattern_start..id+1].to_owned()))
                        }
                    }
                }
            },
            // If pattern matching is disabled and argument matching is also disabled,
            // this will begin the argument matching process.
            (_, false) if !argument_matching  => {
                argument_matching = true;
                argument_start    = id;
            },
            (_, _) => ()
        }
    }

    // In the event that there is leftover data that was not matched, this will add the final
    // string to the token list.
    if pattern_matching {
        tokens.push(Token::Argument(template[pattern_start..].to_owned()));
    } else if argument_matching {
        tokens.push(Token::Argument(template[argument_start..].to_owned()));
    }
}

/// Matches a pattern to it's associated token.
fn match_token(pattern: &str) -> Option<Token> {
    println!("\n{}\n", pattern);
    match pattern {
        "."  => Some(Token::RemoveExtension),
        "#"  => Some(Token::Job),
        "%"  => Some(Token::Slot),
        "/"  => Some(Token::Basename),
        "//" => Some(Token::Dirname),
        "/." => Some(Token::BaseAndExt),
        "#^" => Some(Token::JobTotal),
        _    => None
    }
}

#[test]
fn tokenizer_argument() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "foo");
    assert_eq!(tokens, vec![Token::Argument("foo".to_owned())]);
}

#[test]
fn tokenizer_placeholder() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "{}");
    assert_eq!(tokens, vec![Token::Placeholder]);
}

#[test]
fn tokenizer_remove_extension() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "{.}");
    assert_eq!(tokens, vec![Token::RemoveExtension]);
}

#[test]
fn tokenizer_basename() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "{/}");
    assert_eq!(tokens, vec![Token::Basename]);
}

#[test]
fn tokenizer_dirname() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "{//}");
    assert_eq!(tokens, vec![Token::Dirname]);
}

#[test]
fn tokenizer_base_and_ext() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "{/.}");
    assert_eq!(tokens, vec![Token::BaseAndExt]);
}

#[test]
fn tokenizer_slot() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "{%}");
    assert_eq!(tokens, vec![Token::Slot]);
}

#[test]
fn tokenizer_job() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "{#}");
    assert_eq!(tokens, vec![Token::Job]);
}

#[test]
fn tokenizer_jobtotal() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "{#^}");
    assert_eq!(tokens, vec![Token::JobTotal]);
}

#[test]
fn tokenizer_multiple() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "foo {} bar");
    assert_eq!(tokens, vec![Token::Argument("foo ".to_owned()), Token::Placeholder,
        Token::Argument(" bar".to_owned())]);
}

#[test]
fn tokenizer_no_space() {
    let mut tokens = Vec::new();
    tokenize(&mut tokens, "foo{}bar");
    assert_eq!(tokens, vec![Token::Argument("foo".to_owned()), Token::Placeholder,
        Token::Argument("bar".to_owned())]);
}
