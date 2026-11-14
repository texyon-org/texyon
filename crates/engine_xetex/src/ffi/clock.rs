//! Wall-clock queries and PDF date strings.

use crate::shipout::ShipoutState;
use crate::state::EngineState;

#[cfg(not(windows))]
use libc::{gmtime, localtime};

#[cfg(windows)]
unsafe extern "C" {
    #[link_name = "_gmtime64"]
    fn gmtime(time: *const libc::time_t) -> *mut libc::tm;

    #[link_name = "_localtime64"]
    fn localtime(time: *const libc::time_t) -> *mut libc::tm;
}

/// `gmtime` or `localtime` of `t`.
fn broken_down_time(t: libc::time_t, utc: bool) -> libc::tm {
    // SAFETY: both return null or a pointer to a static `tm` that stays valid
    // until the next call, and it is copied out right away.
    let tm = unsafe {
        if utc { gmtime(&t) } else { localtime(&t) }
            .as_ref()
            .copied()
    };
    tm.expect("time out of range")
}

/// Returns `(minutes, day, month, year)` for `source_date_epoch`.
pub fn get_date_and_time(source_date_epoch: libc::time_t, use_utc: bool) -> (i32, i32, i32, i32) {
    let tm = broken_down_time(source_date_epoch, use_utc);
    (
        tm.tm_hour * 60 + tm.tm_min,
        tm.tm_mday,
        tm.tm_mon + 1,
        tm.tm_year + 1900,
    )
}

impl ShipoutState {
    /// Time since the recorded epoch in units of 1/65536 s, or -1 past 0x7FFF s.
    pub fn get_microinterval(&self) -> i32 {
        let (s, m) = get_seconds_and_micros();
        if (s - self.epochseconds) > 0x7FFF {
            -1
        } else if self.microseconds > m {
            let t = f64::from(s - 1 - self.epochseconds) * 65536.0
                + ((f64::from(m) + 1_000_000.0 - f64::from(self.microseconds)) / 100.0) * 65536.0
                    / 10_000.0;
            t as i32
        } else {
            let t = f64::from(s - self.epochseconds) * 65536.0
                + (f64::from(m - self.microseconds) / 100.0) * 65536.0 / 10_000.0;
            t as i32
        }
    }
}

pub fn get_seconds_and_micros() -> (i32, i32) {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    (elapsed.as_secs() as i32, elapsed.subsec_micros() as i32)
}

/// Formats `t` as a NUL-terminated PDF date string into `time_str`.
pub(crate) fn make_pdf_time(t: libc::time_t, time_str: &mut [u8; 30], utc: bool) {
    let lt = broken_down_time(t, utc);
    let date = format!(
        "D:{}{:02}{:02}{:02}{:02}{:02}",
        i64::from(lt.tm_year) + 1900,
        lt.tm_mon + 1,
        lt.tm_mday,
        lt.tm_hour,
        lt.tm_min,
        lt.tm_sec
    );
    let size = date.len();
    if size >= time_str.len() {
        time_str[0] = 0;
        return;
    }
    time_str[..size].copy_from_slice(date.as_bytes());
    time_str[size] = 0;

    // %S can be 60 or 61 (leap seconds) but PDF expects 00..59.
    if time_str[14] == b'6' {
        time_str[14] = b'5';
        time_str[15] = b'9';
        time_str[16] = 0;
    }

    let gmt = broken_down_time(t, true);
    let mut off = 60 * (lt.tm_hour - gmt.tm_hour) + lt.tm_min - gmt.tm_min;
    if lt.tm_year != gmt.tm_year {
        off += if lt.tm_year > gmt.tm_year {
            1440
        } else {
            -1440
        };
    } else if lt.tm_yday != gmt.tm_yday {
        off += if lt.tm_yday > gmt.tm_yday {
            1440
        } else {
            -1440
        };
    }

    if off == 0 {
        time_str[size] = b'Z';
        time_str[size + 1] = 0;
    } else {
        let off_hours = off / 60;
        let off_mins = (off - off_hours * 60).abs();
        let tz = format!("{:+03}'{:02}'", off_hours, off_mins);
        let tz = &tz.as_bytes()[..tz.len().min(8)];
        time_str[size..size + tz.len()].copy_from_slice(tz);
        time_str[size + tz.len()] = 0;
    }
}

pub(crate) fn nul_terminated(buf: &[u8]) -> &[u8] {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..end]
}

/// Records the PDF date string of `source_date_epoch` for `\creationdate`.
pub fn init_start_time(st: &mut EngineState, source_date_epoch: libc::time_t) {
    let mut buf = [0u8; 30];
    make_pdf_time(source_date_epoch, &mut buf, true);
    st.shipout.start_time_str = nul_terminated(&buf).to_vec();
}
