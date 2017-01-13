use std::path::PathBuf;
use misc::NumToA;

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

pub fn new_job(base: &str, id: usize) -> (usize, String, String) {
    let id = id.to_string();
    let mut stdout = String::from(base) + "/stdout_";
    let mut stderr = String::from(base) + "/stderr_";
    let truncate_value = stdout.len();
    stdout.push_str(&id);
    stderr.push_str(&id);
    println!("stdout: '{}'", stdout);
    (truncate_value, stdout, stderr)
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
