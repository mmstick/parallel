/// Contains all functionality pertaining to parsing, tokenizing, and generating input arguments.
pub mod errors;
mod jobs;
mod man;
mod quote;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::process::exit;
use arrayvec::ArrayVec;
use permutate::Permutator;
use num_cpus;

use super::disk_buffer::{self, DiskBufferTrait};
use super::input_iterator::InputIterator;
use super::tokenizer::Token;
use self::errors::ParseErr;

// Re-export key items from internal modules.
pub use self::errors::{FileErr, InputIteratorErr};


#[derive(PartialEq)]
enum Mode { Arguments, Command, Inputs, Files }

pub const INPUTS_ARE_COMMANDS: u8 = 1;
pub const PIPE_IS_ENABLED:     u8 = 2;
pub const SHELL_ENABLED:       u8 = 4;
pub const QUIET_MODE:          u8 = 8;
pub const VERBOSE_MODE:        u8 = 16;
pub const DASH_EXISTS:         u8 = 32;

/// Defines what quoting mode to use when expanding the command.
enum Quoting { None, Basic, Shell }

/// `Args` is a collection of critical options and arguments that were collected at
/// startup of the application.
pub struct Args<'a> {
    pub flags:        u8,
    pub ncores:       usize,
    pub arguments:    ArrayVec<[Token<'a>; 128]>,
    pub ninputs:      usize,
}

impl<'a> Args<'a> {
    pub fn new() -> Args<'a> {
        Args {
            ncores:       num_cpus::get(),
            flags:        SHELL_ENABLED,
            arguments:    ArrayVec::new(),
            ninputs:      0,
        }
    }

    pub fn parse(&mut self, comm: &mut String, unprocessed_path: &Path) -> Result<InputIterator, ParseErr> {
        let mut quote = Quoting::None;

        // Create a write buffer that automatically writes data to the disk when the buffer is full.
        let mut disk_buffer = disk_buffer::DiskBuffer::new(unprocessed_path).write()
            .map_err(|why| ParseErr::File(FileErr::Open(unprocessed_path.to_owned(), why)))?;

        // Temporary stores for input arguments.
        let mut raw_args                    = env::args().skip(1).peekable();
        let mut lists: Vec<Vec<String>>     = Vec::new();
        let mut current_inputs: Vec<String> = Vec::with_capacity(1024);
        let mut number_of_arguments = 0;

        if env::args().len() > 1 {
            // The purpose of this is to set the initial parsing mode.
            let mut mode = match raw_args.peek().unwrap().as_ref() {
                ":::"  => Mode::Inputs,
                "::::" => Mode::Files,
                _      => Mode::Arguments
            };

            // If there are no arguments to be parsed, then the inputs are commands.
            if mode == Mode::Inputs || mode == Mode::Files {
                self.flags |= INPUTS_ARE_COMMANDS;
            } else {
                self.flags &= 255 ^ INPUTS_ARE_COMMANDS;
            }

            // Parse each and every input argument supplied to the program.
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
                                    self.ncores = if char_iter.peek().is_some() {
                                        // Each character that follows after `j` will be considered an
                                        // input value.
                                        jobs::parse(&argument[2..])?
                                    } else {
                                        // If there was no character after `j`, the following argument
                                        // must be the job value.
                                        let val = &raw_args.next().ok_or(ParseErr::JobsNoValue)?;
                                        jobs::parse(val)?
                                    }
                                } else if character != '-' {
                                    // NOTE: Short mode versions of arguments
                                    for character in argument[1..].chars() {
                                        match character {
                                            'h' => {
                                                println!("{}", man::MAN_PAGE);
                                                exit(0);
                                            },
                                            'n' => self.flags &= 255 ^ SHELL_ENABLED,
                                            'p' => self.flags |= PIPE_IS_ENABLED,
                                            'q' => quote = Quoting::Basic,
                                            's' => self.flags |= QUIET_MODE,
                                            'v' => self.flags |= VERBOSE_MODE,
                                            _ => {
                                                return Err(ParseErr::InvalidArgument(argument.to_owned()))
                                            }
                                        }
                                    }
                                } else {
                                    // NOTE: Long mode versions of arguments
                                    match &argument[2..] {
                                        "help" => {
                                            println!("{}", man::MAN_PAGE);
                                            exit(0);
                                        },
                                        "jobs" => {
                                            let val = &raw_args.next().ok_or(ParseErr::JobsNoValue)?;
                                            self.ncores = jobs::parse(val)?
                                        },
                                        "no-shell" => self.flags &= 255 ^ SHELL_ENABLED,
                                        "num-cpu-cores" => {
                                            println!("{}", num_cpus::get());
                                            exit(0);
                                        },
                                        "pipe" => self.flags |= PIPE_IS_ENABLED,
                                        "quiet" | "silent" => self.flags |= QUIET_MODE,
                                        "quote" => quote = Quoting::Basic,
                                        "shellquote" => quote = Quoting::Shell,
                                        "verbose" => self.flags |= VERBOSE_MODE,
                                        "version" => {
                                            println!("parallel 0.6.2\n\nCrate Dependencies:");
                                            println!("    libc      0.2.15");
                                            println!("    num_cpus  1.0.0");
                                            println!("    permutate 0.1.3");
                                            exit(0);
                                        }
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
                            match argument {
                                ":::" => {
                                    mode = Mode::Inputs;
                                    self.flags |= INPUTS_ARE_COMMANDS;
                                },
                                "::::" => {
                                    mode = Mode::Files;
                                    self.flags |= INPUTS_ARE_COMMANDS;
                                }
                                _ => {
                                    // The command has been supplied, and argument parsing is over.
                                    comm.push_str(argument);
                                    mode = Mode::Command;
                                }
                            }
                        }
                    },
                    Mode::Command => match argument {
                        // Arguments after `:::` are input values.
                        ":::" | ":::+" => mode = Mode::Inputs,
                        // Arguments after `::::` are files with inputs.
                        "::::" | "::::+" => mode = Mode::Files,
                        // All other arguments are command arguments.
                        _ => {
                            comm.push(' ');
                            comm.push_str(argument);
                        }
                    },
                    _ => match argument {
                        // `:::` denotes that the next set of inputs will be added to a new list.
                        ":::"  => {
                            mode = Mode::Inputs;
                            if !current_inputs.is_empty() {
                                lists.push(current_inputs.clone());
                                current_inputs.clear();
                            }
                        },
                        // `:::+` denotes that the next set of inputs will be added to the current list.
                        ":::+" => mode = Mode::Inputs,
                        // `::::` denotes that the next set of inputs will be added to a new list.
                        "::::"  => {
                            mode = Mode::Files;
                            if !current_inputs.is_empty() {
                                lists.push(current_inputs.clone());
                                current_inputs.clear();
                            }
                        },
                        // `:::+` denotes that the next set of inputs will be added to the current list.
                        "::::+" => mode = Mode::Files,
                        // All other arguments will be added to the current list.
                        _ => match mode {
                            Mode::Inputs => current_inputs.push(argument.to_owned()),
                            Mode::Files => file_parse(&mut current_inputs, argument)?,
                            _ => unreachable!()
                        }
                    }
                }
            }

            if !current_inputs.is_empty() {
                lists.push(current_inputs.clone());
            }

            if lists.len() > 1 {
                // Convert the Vec<Vec<String>> into a Vec<Vec<&str>>
                let tmp: Vec<Vec<&str>> = lists.iter()
                    .map(|list| list.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
                    .collect();

                // Convert the Vec<Vec<&str>> into a Vec<&[&str]>
                let list_array: Vec<&[&str]> = tmp.iter().map(AsRef::as_ref).collect();

                // Create a `Permutator` with the &[&[&str]] as the input.
                let permutator = Permutator::new(&list_array[..]);

                for permutation in permutator {
                    let mut iter = permutation.iter();
                    disk_buffer.write(iter.next().unwrap().as_bytes())
                        .map_err(|why| ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;
                    for element in iter {
                        disk_buffer.write_byte(b' ')
                            .map_err(|why| ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;
                        disk_buffer.write(element.as_bytes())
                            .map_err(|why| ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;
                    }
                    disk_buffer.write_byte(b'\n')
                        .map_err(|why| ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;
                    number_of_arguments += 1;
                }
            } else {
                for input in current_inputs {
                    disk_buffer.write(input.as_bytes()).map_err(|why|
                        ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;
                    disk_buffer.write_byte(b'\n').map_err(|why|
                        ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;
                    number_of_arguments += 1;
                }
            }
        }

        // If no inputs are provided, read from stdin instead.
        if disk_buffer.is_empty() {
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                if let Ok(line) = line {
                    disk_buffer.write(line.as_bytes()).map_err(|why|
                        ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;
                    disk_buffer.write_byte(b'\n').map_err(|why|
                        ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;
                    number_of_arguments += 1;
                }
            }
        }

        if number_of_arguments == 0 { return Err(ParseErr::NoArguments); }

        // Flush the contents of the buffer to the disk before tokenizing the command argument.
        disk_buffer.flush().map_err(|why| ParseErr::File(FileErr::Write(disk_buffer.path.clone(), why)))?;

        // Expand the command if quoting is enabled
        match quote {
            Quoting::None  => (),
            Quoting::Basic => quote::basic(comm),
            Quoting::Shell => quote::shell(comm),
        }

        if dash_exists() {
            self.flags |= DASH_EXISTS;
        }

        if !shell_required(&self.arguments) {
            self.flags &= 255 ^ SHELL_ENABLED;
            if self.flags & SHELL_ENABLED == 1 {
                println!("Shell enabled");
            } else {
                println!("Shell disabled");
            }
        }

        // Return an `InputIterator` of the arguments contained within the unprocessed file.
        let inputs = InputIterator::new(unprocessed_path, number_of_arguments).map_err(ParseErr::File)?;
        Ok(inputs)
    }
}

fn shell_required(arguments: &[Token]) -> bool {
    for token in arguments {
        if let &Token::Argument(ref arg) = token {
            if arg.contains(';') || arg.contains('&') {
                return true
            }
        }
    }
    false
}

fn dash_exists() -> bool {
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

/// Attempts to open an input argument and adds each line to the `inputs` list.
fn file_parse<P: AsRef<Path>>(inputs: &mut Vec<String>, path: P) -> Result<(), ParseErr> {
    let path = path.as_ref();
    let file = fs::File::open(path).map_err(|err| ParseErr::File(FileErr::Open(path.to_owned(), err)))?;
    for line in BufReader::new(file).lines() {
        if let Ok(line) = line { inputs.push(line); }
    }
    Ok(())
}
