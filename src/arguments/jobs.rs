use super::ParseErr;
use num_cpus;
/// Receives an input that is either an integer, or percent. If the string ends with `%`, it will
/// be calculated as a percent of the total number of CPU cores. Otherwise, the number provided
/// will be considered the number of jobs to run in parallel.
pub fn parse(value: &str) -> Result<usize, ParseErr> {
    if value.chars().rev().next().unwrap() == '%' {
        // If the last character is `%`, then all but the last character are the value.
        let nchars = value.chars().count();
        if let Ok(percent) = value[0..nchars].parse::<usize>() {
            // No need for floating point math here.
            Ok((num_cpus::get() * percent) / 100)
        } else {
            Err(ParseErr::JobsNaN(value.to_owned()))
        }
    } else {
        if let Ok(ncores) = value.parse::<usize>() {
            Ok(ncores)
        } else {
            Err(ParseErr::JobsNaN(value.to_owned()))
        }
    }
}
