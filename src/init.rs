use std::fs::{read_dir, create_dir_all, remove_file, remove_dir, File};
use std::io::{StderrLock, Write};

use super::arguments::{FileErr, Args};
use super::filepaths;
use super::input_iterator::InputIterator;

fn remove_preexisting_files() -> Result<(), FileErr> {
    // Initialize the base directories of the unprocessed and processed files.
    let path = filepaths::base().ok_or(FileErr::Path)?;

    // Create the directories that are required for storing input files.
    create_dir_all(&path).map_err(|why| FileErr::DirectoryCreate(path.clone(), why))?;

    // Attempt to obtain a listing of all the directories and files within the base directory.
    let directory = read_dir(&path).map_err(|why| FileErr::DirectoryRead(path.clone(), why))?;
    for entry in directory {
        let entry = entry.map_err(|why| FileErr::DirectoryRead(path.clone(), why))?;
        let entry_is_file = entry.file_type().ok().map_or(true, |x| !x.is_dir());
        if entry_is_file {
            remove_file(entry.path()).map_err(|why| FileErr::Remove(path.clone(), why))?;
        } else {
            remove_dir(entry.path()).map_err(|why| FileErr::Remove(path.clone(), why))?;
        }
    }

    // Create empty logs ahead of time
    let errors      = filepaths::errors().ok_or(FileErr::Path)?;
    let unprocessed = filepaths::unprocessed().ok_or(FileErr::Path)?;
    let processed   = filepaths::processed().ok_or(FileErr::Path)?;
    File::create(&errors).map_err(|why| FileErr::Create(errors, why))?;
    File::create(&unprocessed).map_err(|why| FileErr::Create(unprocessed, why))?;
    File::create(&processed  ).map_err(|why| FileErr::Create(processed  , why))?;

    Ok(())
}

pub fn cleanup(stderr: &mut StderrLock) {
    if let Err(why) = remove_preexisting_files() {
        let _ = stderr.write(b"parallel: initialization error: I/O error: ");
        match why {
            FileErr::Create(path, why) => {
                let _ = write!(stderr, "unable to create file: {:?}: {}", path, why);
            }
            FileErr::DirectoryCreate(path, why) => {
                let _ = write!(stderr, "unable to create directory: {:?}: {}", path, why);
            },
            FileErr::DirectoryRead(path, why) => {
                let _ = write!(stderr, "unable to read directory: {:?}: {}", path, why);
            }
            FileErr::Remove(path, why) => {
                let _ = write!(stderr, "unable to remove file: {:?}: {}", path, why);
            },
            _ => unreachable!()
        }
    }
}

pub fn parse(args: &mut Args) -> InputIterator {
    match args.parse() {
        Ok(inputs) => {
            args.ninputs = inputs.total_arguments;
            inputs
        },
        Err(why) => why.handle()
    }
}
