use std::env::home_dir;
use std::path::PathBuf;

pub fn base() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".local/share/parallel");
        path
    })
}

pub fn processed() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".local/share/parallel/processed");
        path
    })
}

pub fn unprocessed() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".local/share/parallel/unprocessed");
        path
    })
}

pub fn errors() -> Option<PathBuf> {
    home_dir().map(|mut path| {
        path.push(".local/share/parallel/errors");
        path
    })
}
