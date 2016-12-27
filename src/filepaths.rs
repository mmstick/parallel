use std::env::home_dir;
use std::path::PathBuf;

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
pub fn job(id: usize) -> (PathBuf, PathBuf) {
    let stdout = PathBuf::from(format!("/tmp/parallel/stdout_{}", id));
    let stderr = PathBuf::from(format!("/tmp/parallel/stderr_{}", id));
    (stdout, stderr)
}

#[cfg(windows)]
pub fn job(id: usize) -> (PathBuf, PathBuf) {
    home_dir().map(|mut stdout| {
        let mut stderr = stdout.clone();
        stdout.push(format!("AppData/Local/Temp/parallel/stdout_{}", id));
        stderr.push(format!("AppData/Local/Temp/parallel/stderr_{}", id));
        (stdout, stderr)
    }).expect("parallel: unable to open home folder")
}
