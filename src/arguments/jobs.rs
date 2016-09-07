use super::ParseErr;
use num_cpus;
/// Receives an input that is either an integer, or percent. If the string ends with `%`, it will
/// be calculated as a percent of the total number of CPU cores. Otherwise, the number provided
/// will be considered the number of jobs to run in parallel.
pub fn parse(value: &str) -> Result<usize, ParseErr> {
    if value.chars().rev().next().unwrap() == '%' {
        // If the last character is `%`, then all but the last character are the value.
        value[0..value.chars().count()].parse::<usize>()
            .map(|percent| (num_cpus::get() * percent) / 100)
            .map_err(|_| ParseErr::JobsNaN(value.to_owned()))

    } else {
        value.parse::<usize>().map_err(|_| ParseErr::JobsNaN(value.to_owned()))
    }
}
