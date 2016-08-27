use std::env;
use tokenizer::{Token, tokenize};

pub struct Args {
    pub ncores:    usize,
    pub grouped:   bool,
    pub arguments: Vec<Token>,
    pub inputs:    Vec<String>
}

pub enum ParseErr {
    JobsNaN(String),
    JobsNoValue,
}


impl Args {
    pub fn parse(&mut self) -> Result<(), ParseErr> {
        let mut parsing_arguments = true;
        let mut command_is_set    = false;
        let mut raw_args = env::args().skip(1).peekable();
        let mut comm = String::new();
        while let Some(argument) = raw_args.next() {
            if parsing_arguments {
                match argument.as_str() {
                    // Defines the number of jobs to run in parallel.
                    "-j" if !command_is_set => {
                        match raw_args.peek() {
                            Some(val) => match val.parse::<usize>() {
                                Ok(val) => self.ncores = val,
                                Err(_)  => return Err(ParseErr::JobsNaN(val.clone()))
                            },
                            None => return Err(ParseErr::JobsNoValue)
                        }
                        let _ = raw_args.next();
                    },
                    "--ungroup" if !command_is_set => {
                        self.grouped = false;
                    }
                    // Arguments after `:::` are input values.
                    ":::" => parsing_arguments = false,
                    _ => {
                        if command_is_set {
                            comm.push(' ');
                            comm.push_str(&argument);
                        } else {
                            comm.push_str(&argument);
                            command_is_set = true;
                        }

                    }
                }
            } else {
                self.inputs.push(argument);
            }
        }

        self.arguments = tokenize(&comm);

        Ok(())
    }
}
