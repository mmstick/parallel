/// Contains all functionality pertaining to parsing, tokenizing, and generating input arguments.
pub mod errors;
mod jobs;
mod man;
mod redirection;

use std::env;
use std::fs::{self, create_dir_all};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::Duration;

use arrayvec::ArrayVec;
use permutate::Permutator;
use tokenizer::Token;
use num_cpus;
use self::errors::ParseErr;

// Re-export key items from internal modules.
pub use self::errors::FileErr;

#[derive(PartialEq)]
enum Mode { Arguments, Command, Inputs, InputsAppend, Files, FilesAppend }

pub const INPUTS_ARE_COMMANDS: u16 = 1;
pub const PIPE_IS_ENABLED:     u16 = 2;
pub const SHELL_ENABLED:       u16 = 4;
pub const QUIET_MODE:          u16 = 8;
pub const VERBOSE_MODE:        u16 = 16;
pub const DASH_EXISTS:         u16 = 32;
pub const DRY_RUN:             u16 = 64;
pub const SHELL_QUOTE:         u16 = 128;
pub const ETA:                 u16 = 256;
pub const JOBLOG:              u16 = 512;
pub const JOBLOG_8601:         u16 = 1024;

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
    pub joblog:    Option<String>,
    pub tempdir:   Option<PathBuf>,
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
            joblog:    None,
            tempdir:   None,
        }
    }

    /// Performs all the work related to parsing program arguments
    pub fn parse(&mut self, comm: &mut String, arguments: &[String], base_path: &mut PathBuf)
        -> Result<usize, ParseErr>
    {
        // Each list will consist of a series of input arguments
        let mut lists: Vec<Vec<String>>     = Vec::new();
        // The `current_inputs` variable will contain all the inputs that have been collected for the first list.
        let mut current_inputs: Vec<String> = Vec::with_capacity(1024);
        // If this value is set, input arguments will be grouped into pairs defined by `max_args` value.
        let mut max_args = 0;
        // It is important for the custom `InputIterator` to know how many input arguments are to be processed.
        let mut number_of_arguments = 0;

        // If no arguments were passed, we can assume that the standard input will be parsing commands.
        // Otherwise, we will parse all the arguments and take actions based on these inputs.
        if env::args().len() > 1 {
            // The first argument defines which `mode` to shift into and which argument `index` to start from.
            let (mut mode, mut index) = match arguments[1].as_str() {
                ":::"  | ":::+"  => (Mode::Inputs, 2),
                "::::" | "::::+" => (Mode::Files, 2),
                _                => (Mode::Arguments, 1)
            };

            // If the `--shebang` parameter was passed, this will be set to `true`.
            let mut shebang = false;

            if let Mode::Arguments = mode {
                // Parse arguments until the command has been found.
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
                                "group" => (),
                                "joblog" => {
                                    let file = arguments.get(index).ok_or(ParseErr::JoblogNoValue)?;
                                    self.joblog = Some(file.to_owned());
                                    index += 1;
                                    self.flags |= JOBLOG;
                                },
                                "joblog-8601" => self.flags |= JOBLOG_8601,
                                "jobs" => {
                                    let val = arguments.get(index).ok_or(ParseErr::JobsNoValue)?;
                                    self.ncores = jobs::parse(val)?;
                                    index += 1;
                                },
                                "line-buffer" | "lb" => (),
                                "num-cpu-cores" => {
                                    println!("{}", num_cpus::get());
                                    exit(0);
                                },
                                "max-args" => {
                                    let val = arguments.get(index).ok_or(ParseErr::MaxArgsNoValue)?;
                                    max_args = val.parse::<usize>().map_err(|_| ParseErr::MaxArgsNaN(index))?;
                                    index += 1;
                                },
                                "mem-free" => {
                                    let val = arguments.get(index).ok_or(ParseErr::MemNoValue)?;
                                    self.memory = parse_memory(val).map_err(|_| ParseErr::MemInvalid(index))?;
                                    index += 1;
                                },
                                "no-notice" => (),
                                "pipe" => self.flags |= PIPE_IS_ENABLED,
                                "quiet" | "silent" => self.flags |= QUIET_MODE,
                                "shellquote" => self.flags |= DRY_RUN + SHELL_QUOTE,
                                "timeout" => {
                                    let val = arguments.get(index).ok_or(ParseErr::TimeoutNoValue)?;
                                    let seconds = val.parse::<f64>().map_err(|_| ParseErr::TimeoutNaN(index))?;
                                    self.timeout = Duration::from_millis((seconds * 1000f64) as u64);
                                    index += 1;
                                },
                                "ungroup" => (),
                                "verbose" => self.flags |= VERBOSE_MODE,
                                "version" => {
                                    println!("MIT/Rust Parallel {}", env!("CARGO_PKG_VERSION"));
                                    exit(0);
                                },
                                "tmpdir" | "tempdir" => {
                                    *base_path = PathBuf::from(arguments.get(index).ok_or(ParseErr::WorkDirNoValue)?);
                                    index += 1;

                                    // Create the base directory if it does not exist
                                    if let Err(why) = create_dir_all(base_path.as_path()) {
                                        let stderr = io::stderr();
                                        let stderr = &mut stderr.lock();
                                        let _ = writeln!(stderr, "parallel: unable to create tempdir {:?}: {}", base_path.as_path(), why);
                                        exit(1);
                                    }
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
                            ":::"  => mode = Mode::Inputs,
                            "::::" => mode = Mode::Files,
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
                        ":::" | ":::+"   => mode = Mode::Inputs,
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

            number_of_arguments = write_inputs_to_disk(lists, current_inputs, max_args, base_path.clone())?;
        } else if let Some(path) = redirection::input_was_redirected() {
            file_parse(&mut current_inputs, path.to_str().ok_or_else(|| ParseErr::RedirFile(path.clone()))?)?;
            number_of_arguments = write_inputs_to_disk(lists, current_inputs, max_args, base_path.clone())?;
        }

        if number_of_arguments == 0 {
            number_of_arguments = write_stdin_to_disk(max_args, base_path.clone())?;
        }

        if number_of_arguments == 0 { return Err(ParseErr::NoArguments); }

        if comm.is_empty() { self.flags |= INPUTS_ARE_COMMANDS; }

        Ok(number_of_arguments)
    }
}

/// Write all arguments from standard input to the disk, recording the number of arguments that were read.
fn write_stdin_to_disk(max_args: usize, mut unprocessed_path: PathBuf) -> Result<usize, ParseErr> {
    println!("parallel: reading inputs from standard input");
    unprocessed_path.push("unprocessed");
    let disk_buffer = fs::OpenOptions::new().truncate(true).write(true).create(true).open(&unprocessed_path)
        .map_err(|why| ParseErr::File(FileErr::Open(unprocessed_path.clone(), why)))?;
    let mut disk_buffer = BufWriter::new(disk_buffer);
    let mut number_of_arguments = 0;

    let stdin = io::stdin();
    if max_args < 2 {
        for line in stdin.lock().lines() {
            if let Ok(line) = line {
                disk_buffer.write(line.as_bytes()).and_then(|_| disk_buffer.write(b"\n"))
                    .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
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
                        .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                } else if max_args_index == 1 {
                    max_args_index = max_args;
                    disk_buffer.write(b" ")
                        .and_then(|_| disk_buffer.write(line.as_bytes()))
                        .and_then(|_| disk_buffer.write(b"\n"))
                        .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                } else {
                    max_args_index -= 1;
                    disk_buffer.write(b" ")
                        .and_then(|_| disk_buffer.write(line.as_bytes()))
                        .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                }
            }
        }
        if max_args_index != max_args {
            disk_buffer.write(b"\n")
                .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
        }
    }

    Ok(number_of_arguments)
}

/// Write all input arguments buffered in memory to the disk, recording the number of arguments that were read.
fn write_inputs_to_disk(lists: Vec<Vec<String>>, current_inputs: Vec<String>, max_args: usize,
    mut unprocessed_path: PathBuf) -> Result<usize, ParseErr>
{
    unprocessed_path.push("unprocessed");
    let disk_buffer = fs::OpenOptions::new().truncate(true).write(true).create(true).open(&unprocessed_path)
        .map_err(|why| ParseErr::File(FileErr::Open(unprocessed_path.to_owned(), why)))?;
    let mut disk_buffer = BufWriter::new(disk_buffer);
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
                .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
            for element in iter {
                disk_buffer.write(b" ").and_then(|_| disk_buffer.write(element.as_bytes()))
                    .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
            }

            number_of_arguments += 1;
        }

        if max_args < 2 {
            disk_buffer.write(b"\n").map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
            while let Ok(true) = permutator.next_with_buffer(&mut permutation_buffer) {
                let mut iter = permutation_buffer.iter();
                disk_buffer.write(iter.next().unwrap().as_bytes())
                    .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                for element in iter {
                    disk_buffer.write(b" ").and_then(|_| disk_buffer.write(element.as_bytes()))
                        .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                }
                disk_buffer.write(b"\n")
                    .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
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
                        .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;

                    for element in iter {
                        disk_buffer.write(b" ").and_then(|_| disk_buffer.write(element.as_bytes()))
                            .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                    }
                } else if max_args_index == 1 {
                    max_args_index = max_args;
                    disk_buffer.write(b" ")
                        .and_then(|_| disk_buffer.write(iter.next().unwrap().as_bytes()))
                        .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;

                    for element in iter {
                        disk_buffer.write(b" ").and_then(|_| disk_buffer.write(element.as_bytes()))
                            .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                    }

                    disk_buffer.write(b"\n")
                        .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                } else {
                    max_args_index -= 1;
                    disk_buffer.write(b" ")
                        .and_then(|_| disk_buffer.write(iter.next().unwrap().as_bytes()))
                        .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;

                    for element in iter {
                        disk_buffer.write(b" ").and_then(|_| disk_buffer.write(element.as_bytes()))
                            .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                    }
                }
            }
        }
    } else if max_args < 2 {
        for input in current_inputs {
            disk_buffer.write(input.as_bytes())
                .and_then(|_| disk_buffer.write(b"\n"))
                .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
            number_of_arguments += 1;
        }
    } else {
        for chunk in current_inputs.chunks(max_args) {
            let max_index = chunk.len()-1;
            let mut index = 0;
            number_of_arguments += 1;

            while index != max_index {
                disk_buffer.write(chunk[index].as_bytes())
                    .and_then(|_| disk_buffer.write(b" "))
                    .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
                index += 1;
            }
            disk_buffer.write(chunk[max_index].as_bytes())
                .and_then(|_| disk_buffer.write(b"\n"))
                .map_err(|why| FileErr::Write(unprocessed_path.clone(), why))?;
        }
    }
    Ok(number_of_arguments)
}

/// Collects all the provided inputs that were passed as command line arguments into the program.
fn parse_inputs(arguments: &[String], mut index: usize, current_inputs: &mut Vec<String>,
    lists: &mut Vec<Vec<String>>, mode: &mut Mode) -> Result<(), ParseErr>
{
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
            ":::"   => switch_mode!(Mode::Inputs),
            // `:::+` denotes that the next set of inputs will be added to the current list.
            ":::+"  => switch_mode!(append Mode::InputsAppend),
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

/// When the `--memfree` option has been selected, this will attempt to parse the unit's value, multiplying
/// that value by the unit's multiplier.
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
        _   => input.parse::<u64>()?
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
    let path       = path.as_ref();
    let file       = fs::File::open(path).map_err(|err| ParseErr::File(FileErr::Open(path.to_owned(), err)))?;
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
