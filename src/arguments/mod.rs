/// Contains all functionality pertaining to parsing, tokenizing, and generating input arguments.
pub mod errors;
mod jobs;
mod man;
mod quote;
mod redirection;

use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::num::ParseIntError;
use std::path::Path;
use std::process::exit;
use std::time::Duration;

use arrayvec::ArrayVec;
use permutate::Permutator;
use num_cpus;

use super::disk_buffer::{self, DiskBufferTrait, DiskBufferWriter};
use super::input_iterator::InputIterator;
use super::tokenizer::Token;
use self::errors::ParseErr;

// Re-export key items from internal modules.
pub use self::errors::{FileErr, InputIteratorErr};

#[derive(PartialEq)]
enum Mode { Arguments, Command, Inputs, InputsAppend, Files, FilesAppend }

pub const INPUTS_ARE_COMMANDS: u16 = 1;
pub const PIPE_IS_ENABLED:     u16 = 2;
pub const SHELL_ENABLED:       u16 = 4;
pub const QUIET_MODE:          u16 = 8;
pub const VERBOSE_MODE:        u16 = 16;
pub const DASH_EXISTS:         u16 = 32;
pub const DRY_RUN:             u16 = 64;
pub const ETA:                 u16 = 128;

/// Defines what quoting mode to use when expanding the command.
enum Quoting { None, Basic, Shell }

/// `Args` is a collection of critical options and arguments that were collected at
/// startup of the application.
pub struct Args {
    pub flags:     u16,
    pub ncores:    usize,
    pub ninputs:   usize,
    pub memory:    u64,
    pub delay:     Duration,
    pub timeout:   Duration,
    pub arguments: ArrayVec<[Token; 128]>,
}

impl Args {
    pub fn new() -> Args {
        Args {
            ncores:    num_cpus::get(),
            flags:     0,
            arguments: ArrayVec::new(),
            ninputs:   0,
            memory:    0,
            delay:     Duration::from_millis(0),
            timeout:   Duration::from_millis(0),
        }
    }

    pub fn parse(&mut self, comm: &mut String, arguments: &[String], unprocessed_path: &Path) -> Result<InputIterator, ParseErr> {
        // Create a write buffer that automatically writes data to the disk when the buffer is full.
        let mut disk_buffer = disk_buffer::DiskBuffer::new(unprocessed_path).write()
            .map_err(|why| ParseErr::File(FileErr::Open(unprocessed_path.to_owned(), why)))?;

        // Temporary stores for input arguments.
        let mut lists: Vec<Vec<String>>     = Vec::new();
        let mut current_inputs: Vec<String> = Vec::with_capacity(1024);
        let mut number_of_arguments = 0;
        let mut max_args = 0;
        let mut quote = Quoting::None;

        if env::args().len() > 1 {
            // The purpose of this is to set the initial parsing mode.
            let (mut mode, mut index) = match arguments[1].as_str() {
                ":::"  => { self.flags |= INPUTS_ARE_COMMANDS; (Mode::Inputs, 2) },
                "::::" => { self.flags |= INPUTS_ARE_COMMANDS; (Mode::Files, 2) },
                _  => (Mode::Arguments, 1)
            };

            let mut shebang = false;

            if let Mode::Arguments = mode {
                while let Some(argument) = arguments.get(index) {
                    index += 1;
                    let mut char_iter = argument.chars();

                    // If the first character is a '-' then it will be processed as an argument.
                    // We can guarantee that there will always be at least one character.
                    if char_iter.next().unwrap() == '-' {
                        // If the second character exists, everything's OK.
                        let character = char_iter.next().ok_or_else(|| ParseErr::InvalidArgument(index-1))?;
                        if character == 'j' {
                            self.ncores = parse_jobs(argument, arguments.get(index), &mut index)?;
                        } else if character == 'n' {
                            max_args = parse_max_args(argument, arguments.get(index), &mut index)?;
                        } else if character != '-' {
                            for character in argument[1..].chars() {
                                match character {
                                    'h' => {
                                        println!("{}", man::MAN_PAGE);
                                        exit(0);
                                    },
                                    'p' => self.flags |= PIPE_IS_ENABLED,
                                    'q' => quote = Quoting::Basic,
                                    's' => self.flags |= QUIET_MODE,
                                    'v' => self.flags |= VERBOSE_MODE,
                                    _ => {
                                        return Err(ParseErr::InvalidArgument(index-1))
                                    }
                                }
                            }
                        } else {
                            // NOTE: Long mode versions of arguments
                            match &argument[2..] {
                                "delay" => {
                                    let val = arguments.get(index).ok_or(ParseErr::DelayNoValue)?;
                                    let seconds = val.parse::<f64>().map_err(|_| ParseErr::DelayNaN(index))?;
                                    self.delay = Duration::from_millis((seconds * 1000f64) as u64);
                                    index += 1;
                                },
                                "dry-run" => self.flags |= DRY_RUN,
                                "eta" => self.flags |= ETA,
                                "help" => {
                                    println!("{}", man::MAN_PAGE);
                                    exit(0);
                                },
                                "jobs" => {
                                    let val = arguments.get(index).ok_or(ParseErr::JobsNoValue)?;
                                    self.ncores = jobs::parse(val)?;
                                    index += 1;
                                },
                                "num-cpu-cores" => {
                                    println!("{}", num_cpus::get());
                                    exit(0);
                                },
                                "max-args" => {
                                    let val = arguments.get(index).ok_or(ParseErr::MaxArgsNoValue)?;
                                    max_args = val.parse::<usize>().map_err(|_| ParseErr::MaxArgsNaN(index))?;
                                    index += 1;
                                }
                                "mem-free" => {
                                    let val = arguments.get(index).ok_or(ParseErr::MemNoValue)?;
                                    self.memory = parse_memory(val).map_err(|_| ParseErr::MemInvalid(index))?;
                                    index += 1;
                                }
                                "pipe" => self.flags |= PIPE_IS_ENABLED,
                                "quiet" | "silent" => self.flags |= QUIET_MODE,
                                "quote" => quote = Quoting::Basic,
                                "shellquote" => quote = Quoting::Shell,
                                "timeout" => {
                                    let val = arguments.get(index).ok_or(ParseErr::TimeoutNoValue)?;
                                    let seconds = val.parse::<f64>().map_err(|_| ParseErr::TimeoutNaN(index))?;
                                    self.timeout = Duration::from_millis((seconds * 1000f64) as u64);
                                    index += 1;
                                }
                                "verbose" => self.flags |= VERBOSE_MODE,
                                "version" => {
                                    println!("parallel 0.9.0\n\nCrate Dependencies:");
                                    println!("    arrayvec     0.3.20");
                                    println!("    gcc          0.3.41");
                                    println!("    kernel32-sys 0.2.2");
                                    println!("    libc         0.2.18");
                                    println!("    num_cpus     1.2.1");
                                    println!("    nodrop       0.1.8");
                                    println!("    odds         0.2.25");
                                    println!("    permutate    0.2.0");
                                    println!("    sys-info     0.4.1");
                                    println!("    wait-timeout 0.1.3");
                                    println!("    winapi       0.2.8");
                                    println!("    winapi-build 0.1.1");
                                    exit(0);
                                }
                                _ if &argument[2..9] == "shebang" => {
                                        shebang = true;
                                        comm.push_str(&argument[10..]);
                                        break
                                },
                                _ => return Err(ParseErr::InvalidArgument(index-1)),
                            }
                        }
                    } else {
                        match argument.as_str() {
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
                        break
                    }
                }
            }

            if let Some(path) = redirection::input_was_redirected() {
                file_parse(&mut current_inputs, path.to_str().ok_or_else(|| ParseErr::RedirFile(path.clone()))?)?;
            } else if let Mode::Command = mode {
                while let Some(argument) = arguments.get(index) {
                    index += 1;
                    match argument.as_str() {
                        // Arguments after `:::` are input values.
                        ":::" | ":::+" => mode = Mode::Inputs,
                        // Arguments after `::::` are files with inputs.
                        "::::" | "::::+" => mode = Mode::Files,
                        // All other arguments are command arguments.
                        _ => {
                            comm.push(' ');
                            comm.push_str(argument);
                            continue
                        }
                    }
                    break
                }

                if shebang {
                    file_parse(&mut current_inputs, &arguments.last().unwrap())?;
                } else {
                    parse_inputs(arguments, index, &mut current_inputs, &mut lists, &mut mode)?;
                }
            } else {
                parse_inputs(arguments, index, &mut current_inputs, &mut lists, &mut mode)?;
            }

            number_of_arguments = write_inputs_to_disk(lists, current_inputs, max_args, &mut disk_buffer)?;
        } else if let Some(path) = redirection::input_was_redirected() {
            self.flags |= INPUTS_ARE_COMMANDS;
            file_parse(&mut current_inputs, path.to_str().ok_or_else(|| ParseErr::RedirFile(path.clone()))?)?;
            number_of_arguments = write_inputs_to_disk(lists, current_inputs, max_args, &mut disk_buffer)?;
        }

        if disk_buffer.is_empty() {
            number_of_arguments = write_stdin_to_disk(&mut disk_buffer, max_args)?;
        }

        if number_of_arguments == 0 { return Err(ParseErr::NoArguments); }

        // Flush the contents of the buffer to the disk before tokenizing the command argument.
        disk_buffer.flush().map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;

        // Expand the command if quoting is enabled
        match quote {
            Quoting::None  => (),
            Quoting::Basic => *comm = quote::basic(comm.as_str()),
            Quoting::Shell => *comm = quote::shell(comm.as_str()),
        }

        // Return an `InputIterator` of the arguments contained within the unprocessed file.
        let inputs = InputIterator::new(unprocessed_path, number_of_arguments).map_err(ParseErr::File)?;
        Ok(inputs)
    }
}

fn write_stdin_to_disk(disk_buffer: &mut DiskBufferWriter, max_args: usize) -> Result<usize, ParseErr> {
    let mut number_of_arguments = 0;

    let stdin = io::stdin();
    if max_args < 2 {
        for line in stdin.lock().lines() {
            if let Ok(line) = line {
                disk_buffer.write(line.as_bytes()).and_then(|_| disk_buffer.write_byte(b'\n'))
                    .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                number_of_arguments += 1;
            }
        }
    } else {
        let mut max_args_index = max_args;
        for line in stdin.lock().lines() {
            if let Ok(line) = line {
                if max_args_index == max_args {
                    max_args_index -= 1;
                    number_of_arguments += 1;
                    disk_buffer.write(line.as_bytes())
                        .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                } else if max_args_index == 1 {
                    max_args_index = max_args;
                    disk_buffer.write_byte(b' ')
                        .and_then(|_| disk_buffer.write(line.as_bytes()))
                        .and_then(|_| disk_buffer.write_byte(b'\n'))
                        .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                } else {
                    max_args_index -= 1;
                    disk_buffer.write_byte(b' ')
                        .and_then(|_| disk_buffer.write(line.as_bytes()))
                        .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                }
            }
        }
        if max_args_index != max_args {
            disk_buffer.write_byte(b'\n')
                .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
        }
    }

    Ok(number_of_arguments)
}

fn write_inputs_to_disk(lists: Vec<Vec<String>>, current_inputs: Vec<String>, max_args: usize,
    disk_buffer: &mut DiskBufferWriter) -> Result<usize, ParseErr> {
    let mut number_of_arguments = 0;

    if lists.len() > 1 {
        // Convert the Vec<Vec<String>> into a Vec<Vec<&str>>
        let tmp: Vec<Vec<&str>> = lists.iter()
            .map(|list| list.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
            .collect();

        // Convert the Vec<Vec<&str>> into a Vec<&[&str]>
        let list_array: Vec<&[&str]> = tmp.iter().map(AsRef::as_ref).collect();

        // Create a `Permutator` with the &[&[&str]] as the input.
        let mut permutator = Permutator::new(&list_array[..]);

        // Generate the first permutation's buffer
        let mut permutation_buffer = permutator.next().unwrap();
        {
            let mut iter = permutation_buffer.iter();
            disk_buffer.write(iter.next().unwrap().as_bytes())
                .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
            for element in iter {
                disk_buffer.write_byte(b' ').and_then(|_| disk_buffer.write(element.as_bytes()))
                    .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
            }

            number_of_arguments += 1;
        }

        // Reuse that buffer for each successive permutation
        if max_args < 2 {
            disk_buffer.write_byte(b'\n').map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
            while let Ok(true) = permutator.next_with_buffer(&mut permutation_buffer) {
                let mut iter = permutation_buffer.iter();
                disk_buffer.write(iter.next().unwrap().as_bytes())
                    .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                for element in iter {
                    disk_buffer.write_byte(b' ').and_then(|_| disk_buffer.write(element.as_bytes()))
                        .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                }
                disk_buffer.write_byte(b'\n')
                    .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                number_of_arguments += 1;
            }
        } else {
            let mut max_args_index = max_args - 1;
            while let Ok(true) = permutator.next_with_buffer(&mut permutation_buffer) {
                let mut iter = permutation_buffer.iter();
                if max_args_index == max_args {
                    max_args_index -= 1;
                    number_of_arguments += 1;

                    disk_buffer.write(iter.next().unwrap().as_bytes())
                        .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;

                    for element in iter {
                        disk_buffer.write_byte(b' ').and_then(|_| disk_buffer.write(element.as_bytes()))
                            .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                    }
                } else if max_args_index == 1 {
                    max_args_index = max_args;
                    disk_buffer.write_byte(b' ')
                        .and_then(|_| disk_buffer.write(iter.next().unwrap().as_bytes()))
                        .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;

                    for element in iter {
                        disk_buffer.write_byte(b' ').and_then(|_| disk_buffer.write(element.as_bytes()))
                            .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                    }

                    disk_buffer.write_byte(b'\n')
                        .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                } else {
                    max_args_index -= 1;
                    disk_buffer.write_byte(b' ')
                        .and_then(|_| disk_buffer.write(iter.next().unwrap().as_bytes()))
                        .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;

                    for element in iter {
                        disk_buffer.write_byte(b' ').and_then(|_| disk_buffer.write(element.as_bytes()))
                            .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                    }
                }
            }
        }
    } else if max_args < 2 {
        for input in current_inputs {
            disk_buffer.write(input.as_bytes())
                .and_then(|_| disk_buffer.write_byte(b'\n'))
                .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
            number_of_arguments += 1;
        }
    } else {
        for chunk in current_inputs.chunks(max_args) {
            let max_index = chunk.len()-1;
            let mut index = 0;
            number_of_arguments += 1;

            while index != max_index {
                disk_buffer.write(chunk[index].as_bytes())
                    .and_then(|_| disk_buffer.write_byte(b' '))
                    .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
                index += 1;
            }
            disk_buffer.write(chunk[max_index].as_bytes())
                .and_then(|_| disk_buffer.write_byte(b'\n'))
                .map_err(|why| FileErr::Write(disk_buffer.path.clone(), why))?;
        }
    }
    Ok(number_of_arguments)
}

fn parse_inputs(arguments: &[String], mut index: usize, current_inputs: &mut Vec<String>, lists: &mut Vec<Vec<String>>,
    mode: &mut Mode) -> Result<(), ParseErr> {
    let mut append_list = &mut Vec::new();

    macro_rules! switch_mode {
        ($mode:expr) => {{
            match *mode {
                Mode::InputsAppend | Mode::FilesAppend => merge_lists(current_inputs, append_list),
                _ => (),
            }
            *mode = $mode;
            if !current_inputs.is_empty() {
                lists.push(current_inputs.clone());
                current_inputs.clear();
            }
        }};
        (append $mode:expr) => {{
            match *mode {
                Mode::InputsAppend | Mode::FilesAppend => merge_lists(current_inputs, append_list),
                _ => (),
            }
            *mode = $mode;
        }};
    }

    // Parse each and every input argument supplied to the program.
    while let Some(argument) = arguments.get(index) {
        index += 1;
        match argument.as_str() {
            // `:::` denotes that the next set of inputs will be added to a new list.
            ":::"  => switch_mode!(Mode::Inputs),
            // `:::+` denotes that the next set of inputs will be added to the current list.
            ":::+" => switch_mode!(append Mode::InputsAppend),
            // `::::` denotes that the next set of inputs will be added to a new list.
            "::::"  => switch_mode!(Mode::Files),
            // `:::+` denotes that the next set of inputs will be added to the current list.
            "::::+" => switch_mode!(append Mode::FilesAppend),
            // All other arguments will be added to the current list.
            _ => match *mode {
                Mode::Inputs       => current_inputs.push(argument.clone()),
                Mode::InputsAppend => append_list.push(argument.clone()),
                Mode::Files        => file_parse(current_inputs, argument)?,
                Mode::FilesAppend  => file_parse(append_list, argument)?,
                _                  => unreachable!()
            }
        }
    }

    if !append_list.is_empty() {
        match *mode {
            Mode::InputsAppend | Mode::FilesAppend => merge_lists(current_inputs, append_list),
            _ => (),
        }
    }

    if !current_inputs.is_empty() {
        lists.push(current_inputs.clone());
    }

    Ok(())
}

/// Parses the `max_args` value, `-n3` or `-n 3`, and optionally increments the index if necessary.
fn parse_max_args(argument: &str, next_argument: Option<&String>,index: &mut usize) -> Result<usize, ParseErr> {
    if argument.len() > 2 {
        Ok(argument[2..].parse::<usize>().map_err(|_| ParseErr::MaxArgsNaN(*index))?)
    } else {
        *index += 1;
        let argument = next_argument.ok_or(ParseErr::MaxArgsNoValue)?;
        Ok(argument.parse::<usize>().map_err(|_| ParseErr::MaxArgsNaN(*index))?)
    }
}

/// Merges an `append` list to the `original` list, draining the `append` list in the process.
/// Excess arguments will be truncated, and therefore lost.
fn merge_lists(original: &mut Vec<String>, append: &mut Vec<String>) {
    if original.len() > append.len() {
        original.truncate(append.len());
    }
    for (input, element) in original.iter_mut().zip(append.drain(..)) {
        input.push(' ');
        input.push_str(&element);
    }
}

fn parse_memory(input: &str) -> Result<u64, ParseIntError> {
    let result = match input.chars().last().unwrap() {
        'k' => &input[..input.len()-1].parse::<u64>()? * 1_000,
        'K' => &input[..input.len()-1].parse::<u64>()? * 1_024,
        'm' => &input[..input.len()-1].parse::<u64>()? * 1_000_000,
        'M' => &input[..input.len()-1].parse::<u64>()? * 1_048_576,
        'g' => &input[..input.len()-1].parse::<u64>()? * 1_000_000_000,
        'G' => &input[..input.len()-1].parse::<u64>()? * 1_073_741_824,
        't' => &input[..input.len()-1].parse::<u64>()? * 1_000_000_000_000,
        'T' => &input[..input.len()-1].parse::<u64>()? * 1_099_511_627_776,
        'p' => &input[..input.len()-1].parse::<u64>()? * 1_000_000_000_000_000,
        'P' => &input[..input.len()-1].parse::<u64>()? * 1_125_899_906_842_624,
        _ => input.parse::<u64>()?
    };
    Ok(result)
}

/// Parses the jobs value, and optionally increments the index if necessary.
fn parse_jobs(argument: &str, next_argument: Option<&String>, index: &mut usize) -> Result<usize, ParseErr> {
    let ncores = if argument.len() > 2 {
        jobs::parse(&argument[2..])?
    } else {
        *index += 1;
        jobs::parse(next_argument.ok_or(ParseErr::JobsNoValue)?)?
    };

    Ok(ncores)
}

/// Attempts to open an input argument and adds each line to the `inputs` list.
fn file_parse<P: AsRef<Path>>(inputs: &mut Vec<String>, path: P) -> Result<(), ParseErr> {
    let path = path.as_ref();
    let file = fs::File::open(path).map_err(|err| ParseErr::File(FileErr::Open(path.to_owned(), err)))?;
    let mut buffer = BufReader::new(file).lines();
    if let Some(line) = buffer.next() {
        if let Ok(line) = line {
            if !line.is_empty() && !line.starts_with("#!") { inputs.push(line); }
        }
    }
    for line in buffer {
        if let Ok(line) = line {
            if !line.is_empty() { inputs.push(line); }
        }
    }
    Ok(())
}
