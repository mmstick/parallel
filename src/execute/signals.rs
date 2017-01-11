use std::process::ExitStatus;

#[cfg(unix)]
pub fn get(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status.signal().unwrap_or(0)
}

#[cfg(not(unix))]
pub fn get(child: ExitStatus) -> i32 {
    0
}
