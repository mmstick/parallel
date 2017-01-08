use std::fs;
use std::path::PathBuf;

#[cfg(not(unix))]
/// At this time, only operating systems that feature a `proc` filesystem are supported
pub fn input_was_redirected() -> Option<PathBuf> { None }

#[cfg(unix)]
/// On UNIX systems that feature a `proc` filesystem, if `/proc/self/fd/0` points to a
/// location other than `/dev/pts` or `pipe:`, then the standard input has been redirected.
///
/// - **/proc/self/fd/0** is the current process's standard input
/// - **/proc/self/fd/1** is the current process's standard output
/// - **/proc/self/fd/2** is the current process's standard error
pub fn input_was_redirected() -> Option<PathBuf> {
    if let Ok(link) = fs::read_link("/proc/self/fd/0") {
        if !link.to_string_lossy().starts_with("/dev/pts") && !link.to_string_lossy().starts_with("pipe:") {
            return Some(link)
        }
    }
    None
}
