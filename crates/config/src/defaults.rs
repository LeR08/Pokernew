//! Default config files, embedded in the binary from `config/defaults/`.
//!
//! On start-up they are copied into the data folder **without ever
//! overwriting** a file the user already has. JSON Schemas are app-owned:
//! they are always refreshed in `<data>/schemas/` so external editors
//! (e.g. VS Code + Taplo) can validate the user's files against them.

use std::io;
use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir, DirEntry};

/// The whole `config/defaults/` tree.
pub static DEFAULTS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../config/defaults");

/// Name of the schemas sub-folder.
pub const SCHEMAS_DIR: &str = "schemas";

/// Returns the embedded JSON Schema text for `name` (e.g. `"settings"`).
pub fn schema_text(name: &str) -> Option<&'static str> {
    DEFAULTS
        .get_file(format!("{SCHEMAS_DIR}/{name}.schema.json"))
        .and_then(|f| f.contents_utf8())
}

/// Returns an embedded default file, e.g. `"themes/dark.toml"`.
pub fn default_text(rel: &str) -> Option<&'static str> {
    DEFAULTS.get_file(rel).and_then(|f| f.contents_utf8())
}

/// Result of [`install`].
#[derive(Debug, Default)]
pub struct InstallReport {
    /// User config files created because they were missing.
    pub created: Vec<PathBuf>,
}

/// Copies missing defaults into `root` and refreshes the schemas.
pub fn install(root: &Path) -> io::Result<InstallReport> {
    std::fs::create_dir_all(root)?;
    let mut report = InstallReport::default();
    copy_dir(&DEFAULTS, root, &mut report)?;
    Ok(report)
}

fn copy_dir(dir: &Dir<'_>, root: &Path, report: &mut InstallReport) -> io::Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(d) => {
                std::fs::create_dir_all(root.join(d.path()))?;
                copy_dir(d, root, report)?;
            }
            DirEntry::File(f) => {
                let target = root.join(f.path());
                let is_schema = f.path().starts_with(SCHEMAS_DIR);
                if is_schema {
                    if std::fs::read(&target).ok().as_deref() != Some(f.contents()) {
                        std::fs::write(&target, f.contents())?;
                    }
                } else if !target.exists() {
                    std::fs::write(&target, f.contents())?;
                    report.created.push(target);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_creates_then_never_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let r1 = install(tmp.path()).unwrap();
        assert!(r1.created.iter().any(|p| p.ends_with("settings.toml")));
        assert!(tmp.path().join("schemas/settings.schema.json").is_file());

        let settings = tmp.path().join("settings.toml");
        std::fs::write(&settings, "# mine\n").unwrap();
        let r2 = install(tmp.path()).unwrap();
        assert!(r2.created.is_empty());
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), "# mine\n");
    }

    #[test]
    fn deleted_default_is_restored() {
        let tmp = tempfile::tempdir().unwrap();
        install(tmp.path()).unwrap();
        std::fs::remove_file(tmp.path().join("themes/light.toml")).unwrap();
        let r = install(tmp.path()).unwrap();
        assert_eq!(r.created.len(), 1);
    }
}
