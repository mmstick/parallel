use arguments::JOBLOG_8601;
use numtoa::NumToA;
use std::fs::File;
use std::io::{Write, BufWriter};
use time::{at, Timespec};

// Each `JobLog` consists of a single job's statistics ready to be written to the job log file.
pub struct JobLog {
    /// The `job_id` is used to keep jobs written to the job log file in the correct order
    pub job_id:     usize,
    /// The `start_time` is a measurement of when the job started, since the 1970 UNIX epoch
    pub start_time: Timespec,
    /// The `runtime` is the actual time the application ran, in nanoseconds
    pub runtime:    u64,
    /// The `exit_value` contains the exit value that the job's process quit with
    pub exit_value: i32,
    /// The `signal` contains a non-zero value if the job was killed by a signal
    pub signal:     i32,
    /// Contains the configuration parameters for the joblog
    pub flags:      u16,
    /// The actual `command` that was executed for this job
    pub command:    String
}

impl JobLog {
    /// Writes an individual job log to the job log file, efficiently.
    pub fn write_entry(&self, joblog: &mut File, id_buffer: &mut [u8], pad: usize) {
        // 1: JobID
        let mut joblog = BufWriter::new(joblog);
        let mut index = (self.job_id + 1).numtoa(10, id_buffer);
        let _ = joblog.write(&id_buffer[index..]);
        for _ in 0..pad - (20 - index) {
            let _ = joblog.write(b" ");
        }

        // 2: StartTime
        if self.flags & JOBLOG_8601 != 0 {
            // ISO 8601 representation of the time
            let tm = at(self.start_time);
            let _ = write!(joblog, "{}-{:02}-{:02} {:02}:{:02}:{:02}  ", 1900+tm.tm_year, 1+tm.tm_mon,
                tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec);

        } else {
            // Represented in seconds, with two decimal places
            index = self.start_time.sec.numtoa(10, id_buffer);
            let _ = joblog.write(&id_buffer[index..]);
            let _ = joblog.write(b".");
            let decimal = (self.start_time.nsec % 1_000_000_000) / 1_000_000;
            if decimal == 0 {
                let _ = joblog.write(b"000");
            } else {
                index = decimal.numtoa(10, id_buffer);
                match 20 - index {
                    1 => { let _ = joblog.write(b"00"); },
                    2 => { let _ = joblog.write(b"0"); },
                    _ => (),
                };
                let _ = joblog.write(&id_buffer[index..]);
            }
            let _ = joblog.write(b"  ");
        }

        // 3: Runtime in seconds, with up to three decimal places.
        index = (self.runtime / 1_000_000_000).numtoa(10, id_buffer);
        for _ in 0..6 - (20 - index) {
            let _ = joblog.write(b" ");
        }
        let _ = joblog.write(&id_buffer[index..]);
        let _ = joblog.write(b".");
        let decimal = (self.runtime % 1_000_000_000) / 1_000_000;
        if decimal == 0 {
            let _ = joblog.write(b"000");
        } else {
            index = decimal.numtoa(10, id_buffer);
            match 20 - index {
                1 => { let _ = joblog.write(b"00"); },
                2 => { let _ = joblog.write(b"0"); },
                _ => (),
            };
            let _ = joblog.write(&id_buffer[index..]);
        }
        let _ = joblog.write(b"  ");

        // 4: Exit Value
        index = self.exit_value.numtoa(10, id_buffer);
        let _ = joblog.write(&id_buffer[index..]);
        for _ in 0..9 - (20 - index) {
            let _ = joblog.write(b" ");
        }

        // 5: Signal
        index = self.signal.numtoa(10, id_buffer);
        let _ = joblog.write(&id_buffer[index..]);
        for _ in 0..8 - (20 - index) {
            let _ = joblog.write(b" ");
        }

        // 5: Command
        let _ = joblog.write(self.command.as_bytes());
        let _ = joblog.write(b"\n");
    }
}

/// Creates the column headers in the first line of the job log file
pub fn create(file: &mut File, padding: usize, flags: u16) {
    let mut joblog = BufWriter::new(file);

    // Sequence column is at least 10 chars long, counting space separator.
    let id_column_resize = if padding < 10 { 0 } else { padding - 10 };
    let _ = joblog.write(b"Sequence  ");
    for _ in 0..id_column_resize { let _ = joblog.write(b" "); }

    if flags & JOBLOG_8601 != 0 {
        let _ = joblog.write(b"StartTime(ISO-8601)  ");
    } else {
        let _ = joblog.write(b"StartTime(s)    ");
    }


    // Remaining columns, with the runtim column left-padded.
    let _ = joblog.write(b"Runtime(s)  ExitVal  Signal  Command\n");
}
