use std::fs;
use std::path::PathBuf;

#[cfg(not(unix))]
pub fn input_was_redirected() -> Option<PathBuf> { None }

#[cfg(unix)]
pub fn input_was_redirected() -> Option<PathBuf> {
    if let Ok(link) = fs::read_link("/proc/self/fd/0") {
        if !link.to_string_lossy().starts_with("/dev/pts") { return Some(link) }
    }
    None
}
