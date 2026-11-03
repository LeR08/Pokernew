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

/// Text decoded from a file, with a way back to byte offsets in the file.
///
/// Files are expected in UTF-8. Any byte sequence that is not valid UTF-8
/// is decoded as Windows-1252 (the usual encoding of older French Windows
/// files: `€` = 0x80, `é` = 0xE9), so nothing is lost. Because that changes
/// lengths, offsets in `text` are mapped back to file offsets with
/// [`Decoded::byte_offset`] — import offsets must always be file offsets.
pub struct Decoded {
    pub text: String,
    /// `map[i]` = file offset of text byte `i` (only when the file was not pure UTF-8).
    map: Option<Vec<u32>>,
    len: usize,
}

const CP1252_80_9F: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“',
    '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];

fn cp1252(b: u8) -> char {
    match b {
        0x80..=0x9f => CP1252_80_9F[(b - 0x80) as usize],
        _ => b as char,
    }
}

impl Decoded {
    pub fn new(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        match String::from_utf8(bytes) {
            Ok(text) => Decoded { text, map: None, len },
            Err(e) => {
                let bytes = e.into_bytes();
                let mut text = String::with_capacity(len + len / 8);
                let mut map: Vec<u32> = Vec::with_capacity(len + len / 8);
                let mut i = 0;
                while i < bytes.len() {
                    match std::str::from_utf8(&bytes[i..]) {
                        Ok(s) => {
                            push_mapped(&mut text, &mut map, s, i);
                            break;
                        }
                        Err(err) => {
                            let good = err.valid_up_to();
                            // Safe: validated just above.
                            let s = std::str::from_utf8(&bytes[i..i + good]).unwrap_or_default();
                            push_mapped(&mut text, &mut map, s, i);
                            i += good;
                            let bad = err.error_len().unwrap_or(bytes.len() - i);
                            for k in 0..bad {
                                let c = cp1252(bytes[i + k]);
                                for _ in 0..c.len_utf8() {
                                    map.push((i + k) as u32);
                                }
                                text.push(c);
                            }
                            i += bad;
                        }
                    }
                }
                Decoded { text, map: Some(map), len }
            }
        }
    }

    /// True when some bytes were not UTF-8.
    pub fn was_reencoded(&self) -> bool {
        self.map.is_some()
    }

    /// File offset of a text offset (a char boundary, or `text.len()`).
    pub fn byte_offset(&self, text_offset: usize) -> usize {
        match &self.map {
            None => text_offset,
            Some(m) => m.get(text_offset).map(|&b| b as usize).unwrap_or(self.len),
        }
    }
}

fn push_mapped(text: &mut String, map: &mut Vec<u32>, s: &str, base: usize) {
    text.push_str(s);
    map.extend((0..s.len()).map(|k| (base + k) as u32));
}

#[cfg(test)]
mod decode_tests {
    use super::*;

    #[test]
    fn utf8_is_untouched() {
        let d = Decoded::new("buyIn: 0.45€".as_bytes().to_vec());
        assert!(!d.was_reencoded());
        assert_eq!(d.byte_offset(d.text.len()), "buyIn: 0.45€".len());
    }

    #[test]
    fn windows_1252_bytes_are_decoded_and_offsets_map_back() {
        // "0.45€ é\nX" in Windows-1252, then valid UTF-8 "€".
        let mut bytes = b"0.45\x80 \xe9\nX".to_vec();
        bytes.extend("€".as_bytes());
        let d = Decoded::new(bytes.clone());
        assert!(d.was_reencoded());
        assert_eq!(d.text, "0.45€ é\nX€");
        let x = d.text.find('X').unwrap();
        assert_eq!(d.byte_offset(x), 8);
        assert_eq!(d.byte_offset(d.text.len()), bytes.len());
    }
}
