use numtoa::NumToA;
use std::path::PathBuf;

#[cfg(not(windows))]
pub fn base() -> Option<PathBuf> {
    Some(PathBuf::from("/tmp/parallel"))
}

#[cfg(windows)]
pub fn base() -> Option<PathBuf> {
    use std::env::home_dir;
    home_dir().map(|mut path| {
        path.push("AppData/Local/Temp/parallel");
        path
    })
}

pub fn new_job(base: &str, id: usize, buffer: &mut [u8]) -> (usize, String, String) {
    let mut stdout = String::from(base) + "/stdout_";
    let mut stderr = String::from(base) + "/stderr_";
    let truncate_value = stdout.len();
    let start_indice = id.numtoa(10, buffer);
    for byte in &buffer[start_indice..] {
        stdout.push(*byte as char);
        stderr.push(*byte as char);
    }
    (truncate_value, stdout, stderr)
}

pub fn next_job_path(id: usize, truncate: usize, buffer: &mut [u8], stdout: &mut String, stderr: &mut String) {
    stdout.truncate(truncate);
    stderr.truncate(truncate);
    let start_indice = id.numtoa(10, buffer);
    for byte in &buffer[start_indice..] {
        stdout.push(*byte as char);
        stderr.push(*byte as char);
    }
}
