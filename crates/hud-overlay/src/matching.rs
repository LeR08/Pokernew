//! Window ↔ table matching.
//!
//! The room's window-title format is not guessed. A window is a table when
//! its title contains a **key of a table seen in the recently imported
//! hands**: the table name (`SPACE KO(1195603616)#0043`), the tournament id
//! with the table number, or the tournament name with the table number.
//! Titles and keys are compared **without punctuation, spaces or case**, so
//! every variant seen on Winamax matches:
//!
//! * OS window title: `Winamax Monster Stack(1196250156)(#0000)`;
//! * table header: `TRIDENT SPACE KO #0114 - 0,50 € - No Limit Holdem`.
//!
//! Optional per-room restrictions (program name, title regex) come from
//! `rooms/<room>.toml`. When several tables match a window, the longest key wins.

use regex::Regex;
use serde::Serialize;

use crate::WindowInfo;

/// A table with recent hands, from the database.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTable {
    pub room: String,
    pub table_name: String,
    pub tournament_id: Option<String>,
    pub table_number: Option<u32>,
    pub max_seats: u8,
    pub last_hand_at: i64,
}

/// Per-room detection settings.
#[derive(Debug, Clone, Default)]
pub struct RoomRules {
    pub room: String,
    /// Program names (case-insensitive); empty = any program.
    pub processes: Vec<String>,
    /// The title must match this regex, when set.
    pub title_pattern: Option<Regex>,
    pub hud_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableWindow {
    pub window: WindowInfo,
    pub table: ActiveTable,
    /// The text that matched (for the inspector).
    pub matched_by: String,
}

fn keys(t: &ActiveTable) -> Vec<String> {
    let mut k = vec![t.table_name.clone()];
    // The table number exactly as the room writes it ("0114", "044", "0"),
    // so "#0001" can never match "#0000" once punctuation is removed.
    if let Some((head, no)) = t.table_name.rsplit_once('#') {
        let name = head
            .split_once('(')
            .map(|(n, _)| n.trim_end())
            .unwrap_or(head.trim_end());
        // Table header: "TRIDENT SPACE KO #0114 - 0,50 € - No Limit Holdem".
        k.push(format!("{name} #{no}"));
        if let Some(id) = &t.tournament_id {
            k.push(format!("{id}#{no}"));
        }
    }
    k
}

/// Lowercase letters and digits only: "Monster Stack(1196250156)(#0000)" -> "monsterstack11962501560000".
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn allowed(w: &WindowInfo, r: &RoomRules) -> bool {
    let proc_ok = r.processes.is_empty()
        || r.processes
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&w.process));
    let title_ok = r
        .title_pattern
        .as_ref()
        .is_none_or(|re| re.is_match(&w.title));
    proc_ok && title_ok
}

/// Matches visible windows with active tables. A table is matched at most once.
pub fn match_tables(
    windows: &[WindowInfo],
    tables: &[ActiveTable],
    rules: &[RoomRules],
) -> Vec<TableWindow> {
    let mut out: Vec<TableWindow> = Vec::new();
    for w in windows
        .iter()
        .filter(|w| w.visible && !w.minimized && w.client.w > 100 && w.client.h > 100)
    {
        let title = normalize(&w.title);
        let mut best: Option<(usize, &ActiveTable, String)> = None;
        for t in tables {
            let Some(rule) = rules.iter().find(|r| r.room == t.room) else {
                continue;
            };
            if !allowed(w, rule) {
                continue;
            }
            for k in keys(t) {
                let nk = normalize(&k);
                if nk.len() >= 6
                    && title.contains(&nk)
                    && best.as_ref().is_none_or(|(len, _, _)| nk.len() > *len)
                {
                    best = Some((nk.len(), t, k));
                }
            }
        }
        if let Some((_, t, k)) = best {
            if !out
                .iter()
                .any(|m| m.table.table_name == t.table_name && m.table.room == t.room)
            {
                out.push(TableWindow {
                    window: w.clone(),
                    table: t.clone(),
                    matched_by: k,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rect;

    fn win(id: u64, title: &str, process: &str) -> WindowInfo {
        WindowInfo {
            id,
            title: title.into(),
            class: "Qt5QWindowIcon".into(),
            process: process.into(),
            client: Rect {
                x: 0,
                y: 0,
                w: 800,
                h: 560,
            },
            scale: 1.0,
            minimized: false,
            visible: true,
        }
    }

    fn table(name: &str, id: &str, no: u32) -> ActiveTable {
        ActiveTable {
            room: "winamax".into(),
            table_name: name.into(),
            tournament_id: Some(id.into()),
            table_number: Some(no),
            max_seats: 6,
            last_hand_at: 0,
        }
    }

    fn rules() -> Vec<RoomRules> {
        vec![RoomRules {
            room: "winamax".into(),
            hud_allowed: true,
            ..Default::default()
        }]
    }

    #[test]
    fn title_containing_the_table_name() {
        let tables = [
            table("SPACE KO(1195603616)#0043", "1195603616", 43),
            table("TRIDENT SPACE KO(1195602081)#0053", "1195602081", 53),
        ];
        let wins = [
            win(
                1,
                "Winamax - SPACE KO(1195603616)#0043 - 250/500",
                "winamax.exe",
            ),
            win(
                2,
                "Winamax - TRIDENT SPACE KO(1195602081)#0053",
                "winamax.exe",
            ),
            win(3, "Lobby Winamax", "winamax.exe"),
            win(4, "Notes - SPACE KO", "notepad.exe"),
        ];
        let m = match_tables(&wins, &tables, &rules());
        let pairs: Vec<(u64, &str)> = m
            .iter()
            .map(|x| (x.window.id, x.table.table_name.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                (1, "SPACE KO(1195603616)#0043"),
                (2, "TRIDENT SPACE KO(1195602081)#0053")
            ]
        );
    }

    #[test]
    fn real_winamax_title() {
        // Title copied from the user's screenshot (3 Nov. 2026).
        let tables = [
            table("TRIDENT SPACE KO(1195602081)#0114", "1195602081", 114),
            table("TRIDENT SPACE KO(1195602081)#0053", "1195602081", 53),
        ];
        let m = match_tables(
            &[win(
                7,
                "TRIDENT SPACE KO #0114 - 0,50 € - No Limit Holdem",
                "Winamax.exe",
            )],
            &tables,
            &rules(),
        );
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].table.table_name, "TRIDENT SPACE KO(1195602081)#0114");
        assert_eq!(m[0].matched_by, "TRIDENT SPACE KO #0114");
    }

    #[test]
    fn real_winamax_os_window_title() {
        // From the user's window inspector (3 Nov. 2026).
        let tables = [table("Monster Stack(1196250156)#0000", "1196250156", 0)];
        let m = match_tables(
            &[win(
                8,
                "Winamax Monster Stack(1196250156)(#0000)",
                "Winamax.exe",
            )],
            &tables,
            &rules(),
        );
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].matched_by, "Monster Stack(1196250156)#0000");
    }

    #[test]
    fn another_table_number_does_not_match() {
        let tables = [table("Monster Stack(1196250156)#0001", "1196250156", 1)];
        assert!(match_tables(
            &[win(
                8,
                "Winamax Monster Stack(1196250156)(#0000)",
                "Winamax.exe"
            )],
            &tables,
            &rules()
        )
        .is_empty());
    }

    #[test]
    fn longest_key_wins_and_case_is_ignored() {
        // "SPACE KO(…)" is contained in "TRIDENT SPACE KO(…)" only if ids match; make them clash on purpose.
        let tables = [
            table("KO(1)#0001", "1", 1),
            table("SPACE KO(1)#0001", "1", 1),
        ];
        let m = match_tables(&[win(1, "space ko(1)#0001", "x.exe")], &tables, &rules());
        assert_eq!(m[0].table.table_name, "SPACE KO(1)#0001");
    }

    #[test]
    fn tournament_id_and_table_number() {
        let tables = [table("SPACE KO(1195603616)#0043", "1195603616", 43)];
        let m = match_tables(
            &[win(9, "Table 1195603616 - #0043", "w.exe")],
            &tables,
            &rules(),
        );
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].matched_by, "1195603616#0043");
    }

    #[test]
    fn process_and_title_rules_restrict() {
        let tables = [table("SPACE KO(1195603616)#0043", "1195603616", 43)];
        let wins = [win(1, "SPACE KO(1195603616)#0043", "chrome.exe")];
        let mut r = rules();
        r[0].processes = vec!["Winamax.exe".into()];
        assert!(match_tables(&wins, &tables, &r).is_empty());
        r[0].processes.clear();
        r[0].title_pattern = Some(Regex::new("^Winamax").unwrap());
        assert!(match_tables(&wins, &tables, &r).is_empty());
    }

    #[test]
    fn minimized_and_tiny_windows_are_ignored() {
        let tables = [table("T(1)#0001", "1", 1)];
        let mut a = win(1, "T(1)#0001", "w.exe");
        a.minimized = true;
        let mut b = win(2, "T(1)#0001", "w.exe");
        b.client.w = 50;
        assert!(match_tables(&[a, b], &tables, &rules()).is_empty());
    }

    #[test]
    fn a_table_is_matched_once() {
        let tables = [table("T(1)#0001", "1", 1)];
        let m = match_tables(
            &[
                win(1, "T(1)#0001", "w.exe"),
                win(2, "T(1)#0001 (2)", "w.exe"),
            ],
            &tables,
            &rules(),
        );
        assert_eq!(m.len(), 1);
    }
}
