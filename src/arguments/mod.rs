use std::env;
use std::io::{self, BufRead, BufReader};
use std::process::exit;
use tokenizer::{Token, tokenize};
use num_cpus;
mod jobs;
mod man;

use std::fs;

/// `Args` is a collection of critical options and arguments that were collected at
/// startup of the application.
pub struct Args {
    /// The number of jobs to run in parallel.
    pub ncores:     usize,
    /// Whether stdout/stderr of each job should be handled serially or not.
    ///
    /// **NOTE:** This has a performance cost when enabled.
    pub grouped:    bool,
    /// Whether the platform's shell should be used or not.
    ///
    /// **NOTE:** This has a large performance cost.
    pub uses_shell: bool,
    /// If set to true, the application will print information about running tasks.
    pub verbose:    bool,
    /// The command arguments collected as a list of `Token`s
    pub arguments:  Vec<Token>,
    /// The inputs supplied that will be used with `arguments`.
    pub inputs:     Vec<String>
}

/// The error type for the argument module.
pub enum ParseErr {
    /// An error occured opening an input file.
    InputFileError(String, String),
    /// The value supplied for `--jobs` is not a number.
    JobsNaN(String),
    /// No value was supplied for '--jobs'
    JobsNoValue,
    /// The argument supplied is not a valid argument.
    InvalidArgument(String),
}

enum Mode {
    Arguments,
    Command,
    Inputs,
    Files
}


impl Args {
    pub fn parse(&mut self) -> Result<(), ParseErr> {
        let mut mode = Mode::Arguments;
        let mut raw_args = env::args().skip(1).peekable();
        let mut comm = String::with_capacity(2048);
        while let Some(argument) = raw_args.next() {
            let argument = argument.as_str();
            match mode {
                Mode::Arguments => {
                    let mut char_iter = argument.chars().peekable();

                    // If the first character is a '-' then it will be processed as an argument.
                    // We can guarantee that there will always be at least one character.
                    if char_iter.next().unwrap() == '-' {
                        // If the second character exists, everything's OK.
                        if let Some(character) = char_iter.next() {
                            // This scope of code allows users to utilize the GNU style
                            // command line arguments, to allow for laziness.
                            if character == 'j' {
                                // The short-hand job argument needs to be handled specially.
                                if char_iter.peek().is_some() {
                                    // Each character that follows after `j` will be considered an
                                    // input value.
                                    match jobs::parse(&argument[2..]) {
                                        Ok(ncores) => self.ncores = ncores,
                                        Err(err)   => return Err(err)
                                    }
                                } else {
                                    // If there wasn't a character after `j`, the argument must
                                    // be supplied as the following argument.
                                    match raw_args.peek() {
                                        Some(val) => match jobs::parse(val) {
                                            Ok(ncores) => self.ncores = ncores,
                                            Err(err)   => return Err(err)
                                        },
                                        None => return Err(ParseErr::JobsNoValue)
                                    }
                                    let _ = raw_args.next();
                                }
                            } else if character != '-' {
                                // All following characters will be considered their own argument.
                                for character in argument[1..].chars() {
                                    match character {
                                        'h' => {
                                            println!("{}", man::MAN_PAGE);
                                            exit(0);
                                        },
                                        'n' => self.uses_shell = false,
                                        'u' => self.grouped = false,
                                        'v' => self.verbose = true,
                                        _ => {
                                            return Err(ParseErr::InvalidArgument(argument.to_owned()))
                                        }
                                    }
                                }
                            } else {
                                // These are all the long mode versions of the arguments.
                                match &argument[2..] {
                                    "help" => {
                                        println!("{}", man::MAN_PAGE);
                                        exit(0);
                                    },
                                    "jobs" => {
                                        match raw_args.peek() {
                                            Some(val) => match jobs::parse(val) {
                                                Ok(ncores) => self.ncores = ncores,
                                                Err(err)   => return Err(err)
                                            },
                                            None => return Err(ParseErr::JobsNoValue)
                                        }
                                        let _ = raw_args.next();
                                    },
                                    "ungroup" => self.grouped = false,
                                    "no-shell" => self.uses_shell = false,
                                    "num-cpu-cores" => {
                                        println!("{}", num_cpus::get());
                                        exit(0);
                                    },
                                    "verbose" => self.verbose = true,
                                    _ => {
                                        return Err(ParseErr::InvalidArgument(argument.to_owned()));
                                    }
                                }
                            }
                        } else {
                            // `-` will never be a valid argument
                            return Err(ParseErr::InvalidArgument("-".to_owned()));
                        }
                    } else {
                        // The command has been supplied, and argument parsing is over.
                        comm.push_str(argument);
                        mode = Mode::Command;
                    }
                },
                Mode::Command => match argument {
                    // Arguments after `:::` are input values.
                    ":::" => mode = Mode::Inputs,
                    // Arguments after `::::` are files with input lists.
                    "::::" => mode = Mode::Files,
                    // All other arguments are command arguments.
                    _ => {
                        comm.push(' ');
                        comm.push_str(&argument);
                    }
                },
                _ => match argument {
                    ":::"  => mode = Mode::Inputs,
                    "::::" => mode = Mode::Files,
                    _ => match mode {
                        Mode::Inputs => self.inputs.push(argument.to_owned()),
                        Mode::Files => if let Err(why) = file_parse(&mut self.inputs, argument) {
                            return Err(why)
                        },
                        _ => unreachable!()
                    }
                }
            }
        }

        tokenize(&mut self.arguments, &comm);

        // If no inputs are provided, read from stdin instead.
        if self.inputs.is_empty() {
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                if let Ok(line) = line {
                    self.inputs.push(line)
                }
            }
        }

        Ok(())
    }
}

/// Attempts to open an input argument and adds each line to the `inputs` list.
fn file_parse(inputs: &mut Vec<String>, path: &str) -> Result<(), ParseErr> {
    fs::File::open(path)
        .map_err(|err| ParseErr::InputFileError(path.to_owned(), err.to_string()))
        .map(|file| {
            for line in BufReader::new(file).lines() {
                if let Ok(line) = line {
                    inputs.push(line);
                }
            }
            ()
        })
}
