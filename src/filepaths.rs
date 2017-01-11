use std::env::home_dir;
use std::path::PathBuf;
use misc::NumToA;

#[cfg(not(windows))]
pub fn base() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".local/share/parallel");
        path
    })
}

#[cfg(windows)]
pub fn base() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push("AppData/Local/Temp/parallel");
        path
    })
}

#[cfg(not(windows))]
pub fn processed() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".local/share/parallel/processed");
        path
    })
}

#[cfg(windows)]
pub fn processed() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push("AppData/Local/Temp/parallel/processed");
        path
    })
}

#[cfg(not(windows))]
pub fn unprocessed() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".local/share/parallel/unprocessed");
        path
    })
}

#[cfg(windows)]
pub fn unprocessed() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push("AppData/Local/Temp/parallel/unprocessed");
        path
    })
}


#[cfg(not(windows))]
pub fn errors() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".local/share/parallel/errors");
        path
    })
}

#[cfg(windows)]
pub fn errors() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push("AppData/Local/Temp/parallel/errors");
        path
    })
}

pub fn outputs_path() -> PathBuf {
    PathBuf::from("/tmp/parallel/")
}

#[cfg(not(windows))]
pub fn new_job(id: usize) -> (usize, String, String) {
    let id = id.to_string();
    let mut stdout = String::from("/tmp/parallel/stdout_");
    let mut stderr = String::from("/tmp/parallel/stderr_");
    let truncate_value = stdout.len();
    stdout.push_str(&id);
    stderr.push_str(&id);
    (truncate_value, stdout, stderr)
}

#[cfg(windows)]
pub fn new_job(id: usize, buffer: &mut [u8; 64]) -> (usize, String, String) {
    home_dir().map(|home| {
        let mut stdout = home.to_str().unwrap().to_owned();
        let mut stderr = stdout.clone();
        stdout.push_str("AppData/Local/Temp/parallel/stdout_");
        stderr.push_str("AppData/Local/Temp/parallel/stderr_");
        let truncate_value = stdout.len();

        let length = itoa(buffer, id, 10);
        for byte in &buffer[0..length] {
            stdout.push(*byte as char);
            stderr.push(*byte as char);
        }

        (truncate_value, stdout, stderr)
    }).expect("parallel: unable to open home folder")
}

pub fn next_job_path(id: usize, truncate: usize, buffer: &mut [u8; 64], stdout: &mut String, stderr: &mut String) {
    stdout.truncate(truncate);
    stderr.truncate(truncate);
    let length = id.numtoa(10, buffer);
    for byte in &buffer[0..length] {
        stdout.push(*byte as char);
        stderr.push(*byte as char);
    }
}
