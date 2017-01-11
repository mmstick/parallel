use misc::NumToA;
use std::fs::File;
use std::io::{Write, BufWriter};
use time::Timespec;

pub struct JobLog {
    pub job_id:     usize,
    pub start_time: Timespec,
    pub runtime:    u64,
    pub exit_value: i32,
    pub signal:     i32,
    pub command:    String
}

impl JobLog {
    pub fn write_entry(&self, joblog: &mut File, id_buffer: &mut [u8], pad: usize) {
        // 1: JobID
        let mut joblog = BufWriter::new(joblog);
        let bytes_written = (self.job_id + 1).numtoa(10, id_buffer);
        let _ = joblog.write(&id_buffer[0..bytes_written]);
        for _ in 0..pad-bytes_written {
            let _ = joblog.write(b" ");
        }

        // 2: StartTime in seconds
        let bytes_written = (self.start_time.sec as u64).numtoa(10, id_buffer);
        let _ = joblog.write(&id_buffer[0..bytes_written]);
        let _ = joblog.write(b".");
        let decimal = (self.start_time.nsec as u64 % 1_000_000_000) / 1_000_000;
        if decimal == 0 {
            let _ = joblog.write(b"000");
        } else {
            let bytes_written = decimal.numtoa(10, id_buffer);
            match bytes_written {
                1 => { let _ = joblog.write(b"00"); },
                2 => { let _ = joblog.write(b"0"); },
                _ => (),
            };
            let _ = joblog.write(&id_buffer[0..bytes_written]);
        }
        for _ in 0..16-(bytes_written+4) {
            let _ = joblog.write(b" ");
        }

        // 3: Runtime in seconds
        let bytes_written = (self.runtime / 1_000_000_000).numtoa(10, id_buffer);
        for _ in 0..10-(bytes_written + 4) {
            let _ = joblog.write(b" ");
        }
        let _ = joblog.write(&id_buffer[0..bytes_written]);
        let _ = joblog.write(b".");
        let decimal = (self.runtime % 1_000_000_000) / 1_000_000;
        if decimal == 0 {
            let _ = joblog.write(b"000");
        } else {
            let bytes_written = decimal.numtoa(10, id_buffer);
            match bytes_written {
                1 => { let _ = joblog.write(b"00"); },
                2 => { let _ = joblog.write(b"0"); },
                _ => (),
            };
            let _ = joblog.write(&id_buffer[0..bytes_written]);
        }
        let _ = joblog.write(b"  ");

        // 4: Exit Value
        let bytes_written = self.exit_value.numtoa(10, id_buffer);
        let _ = joblog.write(&id_buffer[0..bytes_written]);
        for _ in 0..9-bytes_written {
            let _ = joblog.write(b" ");
        }

        // 5: Signal
        let bytes_written = self.signal.numtoa(10, id_buffer);
        let _ = joblog.write(&id_buffer[0..bytes_written]);
        for _ in 0..8-bytes_written {
            let _ = joblog.write(b" ");
        }

        // 5: Command
        let _ = joblog.write(self.command.as_bytes());
        let _ = joblog.write(b"\n");
    }
}

pub fn create(file: &mut File, padding: usize) {
    let mut joblog = BufWriter::new(file);

    // Sequence column is at least 10 chars long, counting space separator.
    let id_column_resize = if padding < 10 { 0 } else { padding - 10 };
    let _ = joblog.write(b"Sequence  ");
    for _ in 0..id_column_resize { let _ = joblog.write(b" "); }

    // StartTime column is always 17 chars long
    let _ = joblog.write(b"StartTime(s)    ");

    // Remaining columns, with the runtim column left-padded.
    let _ = joblog.write(b"Runtime(s)  ExitVal  Signal  Command\n");
}
