use super::errors::ParseErr;
use num_cpus;
/// Receives an input that is either an integer, or percent. If the string ends with `%`, it will
/// be calculated as a percent of the total number of CPU cores. Otherwise, the number provided
/// will be considered the number of jobs to run in parallel.
pub fn parse(value: &str) -> Result<usize, ParseErr> {
    if value.chars().rev().next().unwrap() == '%' {
        // If the last character is `%`, then all but the last character are the value.
        value[0..value.chars().count()-1].parse::<usize>()
            .map(|percent| (num_cpus::get() * percent) / 100)
            .map_err(|_| ParseErr::JobsNaN(value.to_owned()))

    } else {
        value.parse::<usize>().map_err(|_| ParseErr::JobsNaN(value.to_owned()))
    }
}

#[test]
fn test_parse() {
    let ncores = num_cpus::get();
    assert_eq!((ncores * 50) / 100,  parse("50%" ).unwrap());
    assert_eq!((ncores * 100) / 100, parse("100%").unwrap());
    assert_eq!((ncores * 150) / 100, parse("150%").unwrap());
    assert_eq!(4,                    parse("4"   ).unwrap());
}
