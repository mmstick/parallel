extern crate permutate;
use permutate::Permutator;

use std::env::args;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::process::exit;

fn main() {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();
    let mut interpret_files = false;
    let mut no_delimiters   = false;
    let mut benchmark       = false;

    let mut input = Vec::new();
    for argument in args().skip(1) {
        match argument.as_str() {
            "--benchmark" => benchmark = true,
            "-f" => interpret_files = true,
            "-h" => {
                let _ = stdout.write(HELP.as_bytes());
                exit(0);
            },
            "-n" => no_delimiters = true,
            _ => input.push(argument)
        }
    }

    let mut list_vector = Vec::new();
    match parse_arguments(&mut list_vector, &input.join(" "), interpret_files) {
        Ok(_) => {
            // Convert the Vec<Vec<String>> into a Vec<Vec<&str>>
            let tmp: Vec<Vec<&str>> = list_vector.iter()
                .map(|list| list.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
                .collect();

            // Convert the Vec<Vec<&str>> into a Vec<&[&str]>
            let list_array: Vec<&[&str]> = tmp.iter().map(AsRef::as_ref).collect();

            // Create a `Permutator` with the &[&[&str]] as the input.
            let permutator = Permutator::new(&list_array[..]);

            if benchmark {
                let _ = permutator.count();
            } else {
                if no_delimiters {
                    for permutation in permutator {
                        for element in permutation {
                            let _ = stdout.write(element.as_bytes());
                        }
                        let _ = stdout.write(b"\n");
                    }
                } else {
                    for permutation in permutator {
                        let mut permutation = permutation.iter();
                        let first_element: &str = permutation.next().unwrap();
                        let _ = stdout.write(first_element.as_bytes());

                        for element in permutation {
                            let _ = stdout.write(b" ");
                            let _ = stdout.write(element.as_bytes());
                        }
                        let _ = stdout.write(b"\n");
                    }
                }
            }
        },
        Err(why) => {
            let _ = stderr.write(b"permutate: parse error: ");
            match why {
                InputError::FileError(path, why) => {
                    let _ = stderr.write(path.as_bytes());
                    let _ = stderr.write(b" could not be read: ");
                    let _ = stderr.write(why.as_bytes());
                    let _ = stderr.write(b".\n");
                }
                InputError::NoInputsProvided => {
                    let _ = stderr.write(b"no input was provided after separator.\n");
                },
                InputError::NotEnoughInputs  => {
                    let _ = stderr.write(b"not enough space was provided.\n");
                },
            }
            let _ = stderr.write(b"Example Usage: permutate 1 2 3 ::: 4 5 6 ::: 1 2 3\n");
            exit(1);
        }
    }
}

#[derive(Debug)]
enum InputError {
    FileError(String, String),
    NoInputsProvided,
    NotEnoughInputs
}


/// This is effectively a command-line interpreter designed specifically for this program.
fn parse_arguments(list_collection: &mut Vec<Vec<String>>, input: &str, interpret_files: bool)
    -> Result<(), InputError>
{
    let mut add_to_previous_list = false;
    let mut backslash        = false;
    let mut double_quote     = false;
    let mut single_quote     = false;
    let mut match_set        = false;
    let mut interpret_files  = interpret_files;
    let mut matches          = 0;
    let mut current_list     = Vec::new();
    let mut current_argument = String::new();

    for character in input.chars() {
        if match_set {
            match character {
                '+' => add_to_previous_list = true,
                ' ' => {
                    if matches == 3 {
                        if add_to_previous_list {
                            add_to_previous_list = false;
                        } else {
                            if current_list.is_empty() {
                                return Err(InputError::NoInputsProvided);
                            } else {
                                list_collection.push(current_list.clone());
                                current_list.clear();
                            }
                        }
                        interpret_files = false;
                    } else if matches == 4 {
                        if add_to_previous_list {
                            add_to_previous_list = false;
                        } else {
                            if current_list.is_empty() {
                                return Err(InputError::NoInputsProvided);
                            } else {
                                list_collection.push(current_list.clone());
                                current_list.clear();
                            }
                        }
                        interpret_files = true;
                    } else {
                        for _ in 0..matches { current_argument.push(':'); }
                        current_list.push(current_argument.clone());
                        current_argument.clear();
                    }
                    match_set = false;
                    matches = 0;
                } ,
                ':' if !add_to_previous_list => matches += 1,
                _ => {
                    for _ in 0..matches { current_argument.push(':'); }
                    current_argument.push(character);
                    match_set = false;
                    matches = 0;
                },
            }
        } else if backslash {
            match character {
                '\\' | '\'' | ' ' | '\"' => current_argument.push(character),
                _    => {
                    current_argument.push('\\');
                    current_argument.push(' ');
                },
            }
            backslash = false;
        } else if single_quote {
            match character {
                '\\' => backslash = true,
                '\'' => single_quote = false,
                _    => current_argument.push(character)
            }
        } else if double_quote {
            match character {
                '\\' => backslash = true,
                '\"' => double_quote = false,
                _    => current_argument.push(character)
            }
        } else {
            match character {
                ' ' => {
                    if !current_argument.is_empty() {
                        if interpret_files {
                            for argument in try!(file_parse(&current_argument)) {
                                current_list.push(argument);
                            }
                        } else {
                            current_list.push(current_argument.clone());
                        }
                        current_argument.clear();
                    }
                },
                '\\' => backslash = true,
                '\'' => single_quote = true,
                '\"' => double_quote = true,
                ':' => {
                    match_set = true;
                    matches = 1;
                },
                _ => current_argument.push(character)
            }
        }
    }

    if !current_argument.is_empty() {
        if interpret_files {
            for argument in try!(file_parse(&current_argument)) {
                current_list.push(argument);
            }
        } else {
            current_list.push(current_argument);
        }
    }

    if !current_list.is_empty() {
        list_collection.push(current_list);
    }

    if list_collection.len() == 0 {
        return Err(InputError::NotEnoughInputs)
    } else {
        Ok(())
    }
}

/// Attempts to open an input argument and adds each line to the `inputs` list.
fn file_parse(path: &str) -> Result<Vec<String>, InputError> {
    let mut inputs = Vec::new();
    let file = try!(fs::File::open(path)
        .map_err(|err| InputError::FileError(path.to_owned(), err.to_string())));
    for line in BufReader::new(file).lines() {
        if let Ok(line) = line { inputs.push(line); }
    }
    Ok(inputs)
}

const HELP: &'static str = r#"NAME
    permutate - efficient command-line permutator written in Rust

SYNOPSIS
    permutate [-f | -h] [ARGS... MODE]...

DESCRIPTION
    Permutate is a command-line permutator written in Rust, originally designed for inclusion
    within the Rust implementation of Parallel. Following the UNIX philosophy, permutate has
    additionally been spun into both an application and library project to serve as a standalone
    application. The syntax for permutate is nearly identical to Parallel.

OPTIONS
    --benchmark
        Performs a benchmark by permutation all possible values without printing.

    -f
        The first list of inputs will be interpreted as files.

    -h
        Prints this help information.

    -n
        Disable the spaced deliminters between elements.

MODES
    :::
        All following arguments will be interpreted as arguments.

    :::+
        All following arguments will be appended to the previous list.

    ::::
        All following arguments will be interpreted as files.

    ::::+
        All following arguments from files will be appended to the previous list.

"#;

#[cfg(test)]
mod test {
    use super::parse_arguments;

    #[test]
    fn test_parse_arguments() {
        let mut output = Vec::new();
        let inputs = "A B ::: \"C D\" \\\"EF\\\" ::: five:six seven\\ eight";
        let expected = vec![
            vec!["A".to_owned(), "B".to_owned()],
            vec!["C D".to_owned(), "\"EF\"".to_owned()],
            vec!["five:six".to_owned(), "seven eight".to_owned()]
        ];
        let _ = parse_arguments(&mut output, inputs, false);
        assert_eq!(output, expected);
    }
}
