use std::fmt;
use std::io::{self, Write, stderr, stdout};
use std::path::PathBuf;
use std::process::exit;

/// A list of all the possible errors that may happen when working with files.
#[derive(Debug)]
pub enum FileErr {
    Open(PathBuf, io::Error),
    Read(PathBuf, io::Error),
    Write(PathBuf, io::Error),
}

impl fmt::Display for FileErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FileErr::Open(ref path, ref io)  => write!(f, "unable to open {:?}: {}", path, io),
            FileErr::Read(ref path, ref io)  => write!(f, "unable to read {:?}: {}", path, io),
            FileErr::Write(ref path, ref io) => write!(f, "unable to write {:?}: {}", path, io)
        }
    }
}

/// The error type for the argument module.
#[derive(Debug)]
pub enum ParseErr {
    /// The value of the job delay parameter is not set to a number.
    DelayNaN(usize),
    /// The job delay parameter was not set.
    DelayNoValue,
    /// An error occurred with accessing the unprocessed file.
    File(FileErr),
    /// The joblog parameter was not set.
    JoblogNoValue,
    /// The jobs number parameter was not set to a number.
    JobsNaN(String),
    /// The jobs number parameter was not set.
    JobsNoValue,
    /// An invalid argument flag was provided.
    InvalidArgument(usize),
    /// The value for `max_args` was not set to a number.
    MaxArgsNaN(usize),
    /// No value was provided for the `max_args` flag.
    MaxArgsNoValue,
    /// The memfree parameter was invalid.
    MemInvalid(usize),
    /// The memfree parameter was not set.
    MemNoValue,
    /// No arguments were given, so no action can be taken.
    NoArguments,
    /// The standard input could not be redirected to the given file
    RedirFile(PathBuf),
    /// The timeout parameter was not set to a number.
    TimeoutNaN(usize),
    /// The timeout parameter was not set.
    TimeoutNoValue,
    /// The workdir parameter was not set.
    WorkDirNoValue,
}

impl From<FileErr> for ParseErr {
    fn from(input: FileErr) -> ParseErr { ParseErr::File(input) }
}

impl ParseErr {
    pub fn handle(self, arguments: &[String]) -> ! {
        // Always lock an output buffer before using it.
        let stderr = stderr();
        let stdout = stdout();
        let mut stderr = stderr.lock();
        let stdout = &mut stdout.lock();
        let _ = stderr.write(b"parallel: parsing error: ");
        match self {
            ParseErr::File(file_err) => {
                let _ = writeln!(stderr, "{}", file_err);
            }
            ParseErr::DelayNaN(index) => {
                let _ = write!(stderr, "delay parameter, '{}', is not a number.\n", arguments[index]);
            },
            ParseErr::DelayNoValue => {
                let _ = stderr.write(b"no delay parameter was defined.\n");
            },
            ParseErr::JoblogNoValue => {
                let _ = stderr.write(b"no joblog parameter was defined.\n");
            },
            ParseErr::JobsNaN(value) => {
                let _ = write!(stderr, "jobs parameter, '{}', is not a number.\n", value);
            },
            ParseErr::JobsNoValue => {
                let _ = stderr.write(b"no jobs parameter was defined.\n");
            },
            ParseErr::MaxArgsNaN(index) => {
                let _ = write!(stderr, "groups parameter, '{}', is not a number.\n", arguments[index]);
            },
            ParseErr::MaxArgsNoValue => {
                let _ = stderr.write(b"no groups parameter was defined.\n");
            },
            ParseErr::MemNoValue => {
                let _ = stderr.write(b"no memory parameter was defined.\n");
            },
            ParseErr::MemInvalid(index) => {
                let _ = write!(stderr, "invalid memory value: {}\n", arguments[index]);
            }
            ParseErr::InvalidArgument(index) => {
                let _ = write!(stderr, "invalid argument: {}\n", arguments[index]);
            },
            ParseErr::NoArguments => {
                let _ = write!(stderr, "no input arguments were given.\n");
            },
            ParseErr::RedirFile(path) => {
                let _ = write!(stderr, "an error occurred while redirecting file: {:?}\n", path);
            },
            ParseErr::TimeoutNaN(index) => {
                let _ = write!(stderr, "invalid timeout value: {}\n", arguments[index]);
            },
            ParseErr::TimeoutNoValue => {
                let _ = stderr.write(b"no timeout parameter was defined.\n");
            },
            ParseErr::WorkDirNoValue => {
                let _ = stderr.write(b"no workdir parameter was defined.\n");
            }
        };
        let _ = stdout.write(b"For help on command-line usage, execute `parallel -h`\n");
        exit(1);
    }
}
