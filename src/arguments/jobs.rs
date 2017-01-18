use super::errors::ParseErr;
use num_cpus;

/// Receives an input that is either an integer, or percent. If the string ends with `%`, it will
/// be calculated as a percent of the total number of CPU cores. Otherwise, the number provided
/// will be considered the number of jobs to run in parallel.
pub fn parse(value: &str) -> Result<usize, ParseErr> {
    match (value.bytes().next().unwrap(), value.bytes().last().unwrap()) {
        (b'+', b'%') => {
            value[1..value.bytes().count()-1].parse::<usize>()
            .map(|percent| {
                let ncpus = num_cpus::get();
                ncpus + ((ncpus * percent) / 100)
            })
            .map_err(|_| ParseErr::JobsNaN(value.to_owned()))
        },
        (b'-', b'%') => {
            value[1..value.bytes().count()-1].parse::<usize>()
                .map(|percent| {
                    let ncpus = num_cpus::get();
                    let modifier = (ncpus * percent) / 100;
                    if modifier > ncpus { 1 } else { ncpus - modifier }
                })
                .map_err(|_| ParseErr::JobsNaN(value.to_owned()))
        },
        (_, b'%') => {
            value[0..value.bytes().count()-1].parse::<usize>()
                .map(|percent| (num_cpus::get() * percent) / 100)
                .map_err(|_| ParseErr::JobsNaN(value.to_owned()))
        },
        (b'+', _) => {
            value[1..value.bytes().count()].parse::<usize>()
                .map(|value| num_cpus::get() + value)
                .map_err(|_| ParseErr::JobsNaN(value.to_owned()))
        },
        (b'-', _) => {
            value[1..value.bytes().count()].parse::<usize>()
            .map(|value| {
                let ncpus = num_cpus::get();
                if value > ncpus { 1 } else { ncpus - value }
            })
            .map_err(|_| ParseErr::JobsNaN(value.to_owned()))
        },
        _ => {
            value.parse::<usize>().map_err(|_| ParseErr::JobsNaN(value.to_owned()))
        },
    }
}

#[test]
fn job_parsing() {
    let ncores = num_cpus::get();
    assert_eq!((ncores * 50) / 100,  parse("50%" ).unwrap());
    assert_eq!((ncores * 100) / 100, parse("100%").unwrap());
    assert_eq!((ncores * 150) / 100, parse("150%").unwrap());
    assert_eq!(4,                    parse("4"   ).unwrap());
    assert_eq!((ncores * 150) / 100, parse("+50%").unwrap());
    assert_eq!((ncores * 50) / 100,  parse("-50%").unwrap());
    assert_eq!(ncores - 2,           parse("-2"  ).unwrap());
    assert_eq!(ncores + 2,           parse("+2"  ).unwrap());
}
