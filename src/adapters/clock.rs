//! Hora local sem crates: UTC do sistema + offset do fuso lido uma vez com `date +%z`.

use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Segundos desde a meia-noite local.
pub fn seconds_of_day() -> u32 {
    let utc = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    (utc + utc_offset()).rem_euclid(86_400) as u32
}

// shortcut: o offset é lido só na inicialização; se o fuso mudar (horário de verão), reinicie a waybar.
fn utc_offset() -> i64 {
    static OFFSET: OnceLock<i64> = OnceLock::new();
    *OFFSET.get_or_init(|| {
        Command::new("date")
            .arg("+%z")
            .output()
            .ok()
            .and_then(|o| parse_offset(String::from_utf8_lossy(&o.stdout).trim()))
            .unwrap_or(0)
    })
}

/// "+0130" / "-0300" → segundos.
fn parse_offset(s: &str) -> Option<i64> {
    let (sign, digits) = match s.as_bytes().first()? {
        b'+' => (1, &s[1..]),
        b'-' => (-1, &s[1..]),
        _ => return None,
    };
    if digits.len() != 4 {
        return None;
    }
    let h: i64 = digits[..2].parse().ok()?;
    let m: i64 = digits[2..].parse().ok()?;
    Some(sign * (h * 3600 + m * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets() {
        assert_eq!(parse_offset("-0300"), Some(-10_800));
        assert_eq!(parse_offset("+0530"), Some(19_800));
        assert_eq!(parse_offset("0300"), None);
        assert_eq!(parse_offset("+3"), None);
    }
}
