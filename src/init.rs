use std::fs::{read_dir, create_dir_all, remove_file, remove_dir, File};
use std::io::{StderrLock, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

use super::arguments::{FileErr, Args};
use super::filepaths;

fn remove_preexisting_files() -> Result<(PathBuf, PathBuf, PathBuf), FileErr> {
    // Initialize the base directories of the unprocessed and processed files.
    let path = filepaths::base().ok_or(FileErr::Path)?;

    // Create the directories that are required for storing input files.
    create_dir_all(&path).map_err(|why| FileErr::DirectoryCreate(path.clone(), why))?;
    if cfg!(not(windows)) {
        let outputs_path = filepaths::outputs_path();
        create_dir_all(&outputs_path).map_err(|why| FileErr::DirectoryCreate(outputs_path, why))?;
    }

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
    File::create(&errors).map_err(|why| FileErr::Create(errors.clone(), why))?;
    File::create(&unprocessed).map_err(|why| FileErr::Create(unprocessed.clone(), why))?;
    File::create(&processed).map_err(|why| FileErr::Create(processed.clone(), why))?;

    Ok((unprocessed, processed, errors))
}

pub fn cleanup(stderr: &mut StderrLock) -> (PathBuf, PathBuf, PathBuf) {
    match remove_preexisting_files() {
        Ok(values) => values,
        Err(why) => {
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
            exit(1)
        }
    }
}

pub fn parse(args: &mut Args, comm: &mut String, arguments: &[String], unprocessed: &Path) -> usize {
    match args.parse(comm, arguments, unprocessed) {
        Ok(inputs) => inputs,
        Err(why) => why.handle(arguments)
    }
}
