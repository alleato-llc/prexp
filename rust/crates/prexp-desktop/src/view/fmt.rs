//! Small display formatters shared by the view. Domain-free number-to-string
//! helpers — nothing here knows about processes.

/// A compact human-readable byte count (`0`, `512`, `4.0K`, `1.2G`). Raw bytes
/// under 1K are shown as-is; larger values use one decimal and a unit suffix.
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    if n < 1024 {
        return n.to_string();
    }
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{v:.1}{}", UNITS[i])
}

/// A CPU percentage to one decimal place.
pub fn pct(p: f64) -> String {
    format!("{p:.1}")
}

/// A cumulative CPU time (nanoseconds) as a human duration (`1.2s`, `3m 04s`,
/// `1h 02m`).
pub fn duration_ns(ns: u64) -> String {
    let secs = ns / 1_000_000_000;
    if secs < 60 {
        let millis = (ns % 1_000_000_000) / 1_000_000;
        format!("{secs}.{millis:03}s")
    } else if secs < 3600 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// A Unix-epoch second as a `YYYY-MM-DD HH:MM:SS UTC` string. Deterministic (no
/// local clock/timezone), so it's snapshot-stable. Civil date via Howard
/// Hinnant's `civil_from_days` algorithm.
pub fn unix_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };

    format!("{year:04}-{month:02}-{day:02} {hh:02}:{mm:02}:{ss:02} UTC")
}
