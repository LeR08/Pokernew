//! Finds the hand-history folders of installed poker clients.
//!
//! Candidates are per room; the first-run wizard shows what was found and the
//! user confirms. Base folders are passed in so the logic is testable.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Platform folders used to locate clients.
#[derive(Debug, Clone, Default)]
pub struct BaseDirs {
    /// `%APPDATA%` (Roaming).
    pub app_data: Option<PathBuf>,
    /// `%LOCALAPPDATA%`.
    pub local_app_data: Option<PathBuf>,
    /// The user's Documents folder.
    pub documents: Option<PathBuf>,
}

/// One account found on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundAccount {
    pub room: String,
    /// Screen name, taken from the account folder name.
    pub account: String,
    pub folder: PathBuf,
    /// Number of files in the folder (to show the user what will be imported).
    pub files: usize,
}

fn accounts_in(room: &str, accounts_dir: &Path, sub: &str, out: &mut Vec<FoundAccount>) {
    let Ok(rd) = std::fs::read_dir(accounts_dir) else {
        return;
    };
    for e in rd.flatten() {
        let folder = e.path().join(sub);
        if folder.is_dir() {
            let files = std::fs::read_dir(&folder)
                .map(|r| r.flatten().count())
                .unwrap_or(0);
            let account = e.file_name().to_string_lossy().into_owned();
            if !out.iter().any(|f| f.folder == folder) {
                out.push(FoundAccount {
                    room: room.into(),
                    account,
                    folder,
                    files,
                });
            }
        }
    }
}

/// Every account folder found for every supported room.
pub fn detect_accounts(base: &BaseDirs) -> Vec<FoundAccount> {
    let mut out = Vec::new();
    // Winamax. Seen on the user's PC: %APPDATA%\winamax\documents\accounts\<pseudo>\history
    if let Some(a) = &base.app_data {
        accounts_in(
            "winamax",
            &a.join("winamax").join("documents").join("accounts"),
            "history",
            &mut out,
        );
    }
    // Also documented by Winamax for older clients: Documents\Winamax Poker\accounts\<pseudo>\history
    if let Some(d) = &base.documents {
        accounts_in(
            "winamax",
            &d.join("Winamax Poker").join("accounts"),
            "history",
            &mut out,
        );
    }
    out.sort_by(|a, b| (&a.room, &a.account).cmp(&(&b.room, &b.account)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_winamax_accounts_in_both_locations() {
        let tmp = tempfile_dir();
        let appdata = tmp.join("Roaming");
        let docs = tmp.join("Documents");
        let h1 = appdata.join("winamax/documents/accounts/Tantaculejen/history");
        let h2 = docs.join("Winamax Poker/accounts/Other/history");
        std::fs::create_dir_all(&h1).unwrap();
        std::fs::create_dir_all(&h2).unwrap();
        std::fs::write(h1.join("a.txt"), "x").unwrap();
        // An account folder without history is ignored.
        std::fs::create_dir_all(appdata.join("winamax/documents/accounts/Empty")).unwrap();
        let found = detect_accounts(&BaseDirs {
            app_data: Some(appdata),
            local_app_data: None,
            documents: Some(docs),
        });
        let names: Vec<(&str, usize)> = found
            .iter()
            .map(|f| (f.account.as_str(), f.files))
            .collect();
        assert_eq!(names, [("Other", 0), ("Tantaculejen", 1)]);
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    fn tempfile_dir() -> PathBuf {
        let p = std::env::temp_dir().join(format!("lps-detect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
