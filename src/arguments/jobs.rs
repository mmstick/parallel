use super::ParseErr;
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
    let value1 = "50%";
    let value2 = "100%";
    let value3 = "150%";
    let value4 = "4";
    let expected1 = (ncores * 50) / 100;
    let expected2 = (ncores * 100) / 100;
    let expected3 = (ncores * 150) / 100;
    let expected4 = 4;
    assert_eq!(Ok(expected1), parse(value1));
    assert_eq!(Ok(expected2), parse(value2));
    assert_eq!(Ok(expected3), parse(value3));
    assert_eq!(Ok(expected4), parse(value4));
}
