//! Config families: which files exist, where they live, which schema validates them.
//!
//! A family is either a single file (`settings.toml`) or a folder of files
//! (`themes/*.toml`), where each file's stem is its id. Later phases add
//! families (stats, hud, reports, filters, rooms…) by appending to [`FAMILIES`].

use std::path::{Component, Path, PathBuf};

/// Where a family's files live, relative to the data folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// One file; its id is the family name.
    Single(&'static str),
    /// A folder of `*.toml` files; the id is the file stem.
    Dir(&'static str),
}

/// A kind of config file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    pub name: &'static str,
    pub layout: Layout,
    /// Schema name: `schemas/<schema>.schema.json`.
    pub schema: &'static str,
}

/// All families known in this build.
pub const FAMILIES: &[Family] = &[
    Family { name: "settings", layout: Layout::Single("settings.toml"), schema: "settings" },
    Family { name: "hotkeys", layout: Layout::Single("hotkeys.toml"), schema: "hotkeys" },
    Family { name: "themes", layout: Layout::Dir("themes"), schema: "theme" },
    Family { name: "profiles", layout: Layout::Dir("profiles"), schema: "profile" },
    Family { name: "rooms", layout: Layout::Dir("rooms"), schema: "room" },
    Family { name: "stats", layout: Layout::Dir("stats"), schema: "stats" },
    Family { name: "leaks", layout: Layout::Dir("leaks"), schema: "leaks" },
    Family { name: "notes", layout: Layout::Dir("notes"), schema: "notes" },
    Family { name: "hud", layout: Layout::Dir("hud"), schema: "hud" },
];

/// Looks a family up by name.
pub fn family(name: &str) -> Option<&'static Family> {
    FAMILIES.iter().find(|f| f.name == name)
}

/// Checks that an id is a safe file stem (no path tricks, no extension).
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl Family {
    /// Path of the file holding `id`, relative to the data folder.
    pub fn rel_path(&self, id: &str) -> Option<PathBuf> {
        match self.layout {
            Layout::Single(file) => (id == self.name).then(|| PathBuf::from(file)),
            Layout::Dir(dir) => is_valid_id(id).then(|| Path::new(dir).join(format!("{id}.toml"))),
        }
    }

    /// Lists the ids present on disk.
    pub fn list_ids(&self, root: &Path) -> Vec<String> {
        match self.layout {
            Layout::Single(file) => {
                if root.join(file).is_file() {
                    vec![self.name.to_string()]
                } else {
                    vec![]
                }
            }
            Layout::Dir(dir) => {
                let mut ids: Vec<String> = std::fs::read_dir(root.join(dir))
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter_map(|e| {
                        let p = e.path();
                        (p.extension()? == "toml")
                            .then(|| p.file_stem()?.to_str().map(str::to_string))?
                    })
                    .filter(|id| is_valid_id(id))
                    .collect();
                ids.sort();
                ids
            }
        }
    }
}

/// Maps a path inside the data folder back to `(family, id)`.
pub fn classify(root: &Path, path: &Path) -> Option<(&'static Family, String)> {
    let rel = path.strip_prefix(root).ok()?;
    if rel.extension()? != "toml" {
        return None;
    }
    let parts: Vec<&str> = rel
        .components()
        .map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Option<_>>()?;
    for fam in FAMILIES {
        match (fam.layout, parts.as_slice()) {
            (Layout::Single(file), [name]) if *name == file => {
                return Some((fam, fam.name.to_string()));
            }
            (Layout::Dir(dir), [d, name]) if *d == dir => {
                let id = name.strip_suffix(".toml")?;
                return is_valid_id(id).then(|| (fam, id.to_string()));
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_paths() {
        let root = Path::new("/data");
        let (f, id) = classify(root, Path::new("/data/settings.toml")).unwrap();
        assert_eq!((f.name, id.as_str()), ("settings", "settings"));
        let (f, id) = classify(root, Path::new("/data/themes/dark.toml")).unwrap();
        assert_eq!((f.name, id.as_str()), ("themes", "dark"));
        assert!(classify(root, Path::new("/data/themes/dark.toml.tmp")).is_none());
        assert!(classify(root, Path::new("/data/other/x.toml")).is_none());
        assert!(classify(root, Path::new("/data/themes/sub/x.toml")).is_none());
    }

    #[test]
    fn ids_cannot_escape_the_folder() {
        let f = family("themes").unwrap();
        assert!(f.rel_path("../settings").is_none());
        assert!(f.rel_path("a/b").is_none());
        assert!(f.rel_path("my-theme_2").is_some());
        assert!(family("settings").unwrap().rel_path("other").is_none());
    }
}
