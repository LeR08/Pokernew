//! Range bundles: a whole folder of the range library in one text file.
//!
//! ```text
//! # Local Poker Suite ranges v1
//! [6max/100bb/BTN_open]
//! 22+,A2s+,K9s+
//! [6max/100bb/CO_open]
//! 55+,A8s+
//! ```
//!
//! Names are library paths (`folder/sub/name`), the text under them is range
//! notation (`crate::range`). Importing also accepts the PioSOLVER / GTO+ style
//! `AA:1.0, AKs:0.5` (spaces after the colon are removed).

use lps_equity::Range;

pub const HEADER: &str = "# Local Poker Suite ranges v1";

/// Writes ranges (name, notation) as a bundle.
pub fn export(ranges: &[(String, String)]) -> String {
    let mut out = String::from(HEADER);
    out.push('\n');
    for (name, text) in ranges {
        out.push_str(&format!("[{name}]\n{}\n", text.trim()));
    }
    out
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BundleError {
    #[error("line {0}: range text before any [name]")]
    NoName(usize),
    #[error("[{0}]: {1}")]
    Range(String, String),
    #[error("line {0}: empty name")]
    EmptyName(usize),
}

/// Normalises third-party notation: `AA: 1.0` → `AA:1.0`.
pub fn normalise(text: &str) -> String {
    let mut s = text.replace('\r', "");
    while s.contains(": ") {
        s = s.replace(": ", ":");
    }
    s
}

/// Reads a bundle; every range is checked.
pub fn import(text: &str) -> Result<Vec<(String, String)>, BundleError> {
    let mut out: Vec<(String, String)> = vec![];
    for (no, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some(name) = l.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            let name = name.trim();
            if name.is_empty() {
                return Err(BundleError::EmptyName(no + 1));
            }
            out.push((name.to_string(), String::new()));
            continue;
        }
        let Some(last) = out.last_mut() else {
            return Err(BundleError::NoName(no + 1));
        };
        if !last.1.is_empty() {
            last.1.push(',');
        }
        last.1.push_str(&normalise(l));
    }
    for (name, text) in &out {
        Range::parse(text).map_err(|e| BundleError::Range(name.clone(), e.to_string()))?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_exact() {
        let ranges = vec![
            (
                "6max/100bb/BTN open".to_string(),
                "22+,A2s+,K9s+,QTs+,JTs,ATo+,KJo+".to_string(),
            ),
            (
                "6max/100bb/BB vs BTN/3bet".to_string(),
                "QQ+,AKs,A5s:50%,KQo:0.25,AhKd".to_string(),
            ),
            ("vide".to_string(), String::new()),
        ];
        let text = export(&ranges);
        let back = import(&text).unwrap();
        assert_eq!(back.len(), ranges.len());
        for ((n1, t1), (n2, t2)) in ranges.iter().zip(&back) {
            assert_eq!(n1, n2);
            assert_eq!(
                Range::parse(t1).unwrap().weights,
                Range::parse(t2).unwrap().weights,
                "{n1}"
            );
        }
    }

    #[test]
    fn third_party_notation() {
        let b = import("[pio]\nAA:1.0, AKs: 0.5,\nKK:0.75").unwrap();
        let r = Range::parse(&b[0].1).unwrap();
        assert_eq!(r.combo_count(), 6.0 + 2.0 + 4.5);
        assert!(import("AA\n").is_err());
        assert!(import("[x]\nAKz").is_err());
    }
}
