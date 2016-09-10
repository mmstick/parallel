use man;
use std::env::args;
use std::fs;
use std::io::{BufRead, BufReader, StdoutLock, Write};
use std::process::exit;

#[derive(Debug)]
pub enum InputError {
    FileError(String, String),
    NoInputsProvided,
    NotEnoughInputs
}


/// Scans input arguments for flags that control the behaviour of the program.
pub fn parse_options(stdout: &mut StdoutLock) -> (Vec<String>, bool, bool, bool) {
    let mut input = Vec::new();
    let (mut benchmark, mut interpret_files, mut no_delimiters) = (false, false, false);
    for argument in args().skip(1) {
        match argument.as_str() {
            "-b" | "--benchmark" => benchmark = true,
            "-f" | "--files" => interpret_files = true,
            "-h" | "--help" => {
                let _ = stdout.write(man::MANPAGE.as_bytes());
                exit(0);
            },
            "-n" | "--no-delimiters" => no_delimiters = true,
            _ => input.push(argument)
        }
    }
    (input, benchmark, interpret_files, no_delimiters)
}

/// This is effectively a command-line interpreter designed specifically for this program.
pub fn parse_arguments(list_collection: &mut Vec<Vec<String>>, input: &str, interpret_files: bool)
    -> Result<(), InputError>
{
    let mut add_to_previous_list = false;
    let mut backslash            = false;
    let mut double_quote         = false;
    let mut single_quote         = false;
    let mut match_set            = false;
    let mut interpret_files      = interpret_files;
    let mut matches              = 0;
    let mut current_list         = Vec::new();
    let mut current_argument     = String::new();

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

    if list_collection.len() == 0 || (list_collection.len() == 1 && list_collection[0].len() == 1) {
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
