//! Small helpers shared by parsers.

/// Strips a UTF-8 BOM.
pub fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

/// "0.74" / "1" / "463.20" -> cents. At most two decimals.
pub fn parse_cents(s: &str) -> Option<i64> {
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s, ""),
    };
    if int.is_empty() || !int.bytes().all(|b| b.is_ascii_digit()) || frac.len() > 2 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let i: i64 = int.parse().ok()?;
    let f: i64 = if frac.is_empty() { 0 } else { format!("{frac:0<2}").parse().ok()? };
    Some(i * 100 + f)
}

/// "2026/11/02 18:34:43" (UTC) -> Unix seconds.
pub fn parse_utc(s: &str) -> Option<i64> {
    let (date, time) = s.split_once(' ')?;
    let mut d = date.split('/');
    let (y, m, day): (i64, i64, i64) = (d.next()?.parse().ok()?, d.next()?.parse().ok()?, d.next()?.parse().ok()?);
    let mut t = time.split(':');
    let (hh, mm, ss): (i64, i64, i64) = (t.next()?.parse().ok()?, t.next()?.parse().ok()?, t.next()?.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    Some(days_from_civil(y, m, day) * 86_400 + hh * 3600 + mm * 60 + ss)
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm, proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents() {
        assert_eq!(parse_cents("0.74"), Some(74));
        assert_eq!(parse_cents("1"), Some(100));
        assert_eq!(parse_cents("463.2"), Some(46320));
        assert_eq!(parse_cents("1.255"), None);
        assert_eq!(parse_cents("a"), None);
    }

    #[test]
    fn utc() {
        assert_eq!(parse_utc("1970/01/01 00:00:00"), Some(0));
        // 2026-11-02T18:34:43Z, checked against `date -u -d @1793644483`.
        assert_eq!(parse_utc("2026/11/02 18:34:43"), Some(1_793_644_483));
        assert_eq!(parse_utc("2024/02/29 12:00:00"), Some(1_709_208_000));
    }
}
