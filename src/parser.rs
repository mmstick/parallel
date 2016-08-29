use std::env;
use tokenizer::{Token, tokenize};

pub struct Args {
    pub ncores:     usize,
    pub grouped:    bool,
    pub uses_shell: bool,
    pub arguments:  Vec<Token>,
    pub inputs:     Vec<String>
}

pub enum ParseErr {
    JobsNaN(String),
    JobsNoValue,
}


impl Args {
    pub fn parse(&mut self) -> Result<(), ParseErr> {
        let mut parsing_arguments = true;
        let mut command_mode      = false;
        let mut raw_args = env::args().skip(1).peekable();
        let mut comm = String::with_capacity(2048);
        while let Some(argument) = raw_args.next() {
            if parsing_arguments {
                if command_mode {
                    match argument.as_str() {
                        // Arguments after `:::` are input values.
                        ":::" => parsing_arguments = false,
                        _ => {
                            comm.push(' ');
                            comm.push_str(&argument);
                        }
                    }
                } else {
                    match argument.as_str() {
                        // Defines the number of jobs to run in parallel.
                        "-j" => {
                            match raw_args.peek() {
                                Some(val) => match val.parse::<usize>() {
                                    Ok(val) => self.ncores = val,
                                    Err(_)  => return Err(ParseErr::JobsNaN(val.clone()))
                                },
                                None => return Err(ParseErr::JobsNoValue)
                            }
                            let _ = raw_args.next();
                        },
                        "--ungroup" => self.grouped = false,
                        "--no-shell" => self.uses_shell = false,
                        _ => {
                            comm.push_str(&argument);
                            command_mode = true;
                        }
                    }
                }
            } else {
                self.inputs.push(argument);
            }
        }

        tokenize(&mut self.arguments, &comm);

        Ok(())
    }
}
