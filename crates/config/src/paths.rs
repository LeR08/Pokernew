//! Data folder resolution.
//!
//! Priority (first match wins):
//! 1. `--data-dir <path>` on the command line;
//! 2. the `LPS_DATA_DIR` environment variable;
//! 3. **portable mode**: a `portable.flag` file next to the executable puts the
//!    data in `<exe dir>/data/`;
//! 4. the platform config folder + `LocalPokerSuite`
//!    (`%APPDATA%\LocalPokerSuite` on Windows).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Folder name under the platform config directory.
pub const APP_DIR_NAME: &str = "LocalPokerSuite";
/// File whose presence next to the executable enables portable mode.
pub const PORTABLE_FLAG: &str = "portable.flag";
/// Data sub-folder used in portable mode.
pub const PORTABLE_DATA_DIR: &str = "data";
/// Environment variable overriding the data folder.
pub const ENV_DATA_DIR: &str = "LPS_DATA_DIR";

/// Why a given data folder was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataDirSource {
    CommandLine,
    Environment,
    Portable,
    Default,
}

/// The resolved data folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataDir {
    pub path: PathBuf,
    pub source: DataDirSource,
}

/// Inputs to [`resolve`], gathered from the process so the logic stays pure.
#[derive(Debug, Default, Clone)]
pub struct ResolveInput {
    pub cli: Option<PathBuf>,
    pub env: Option<OsString>,
    pub exe_dir: Option<PathBuf>,
    /// Platform config folder (`dirs::config_dir()`).
    pub platform_config_dir: Option<PathBuf>,
}

impl ResolveInput {
    /// Collects the inputs from the running process.
    pub fn from_process() -> Self {
        let mut args = std::env::args_os().skip(1);
        let mut cli = None;
        while let Some(arg) = args.next() {
            if arg == "--data-dir" {
                cli = args.next().map(PathBuf::from);
            } else if let Some(v) = arg.to_str().and_then(|s| s.strip_prefix("--data-dir=")) {
                cli = Some(PathBuf::from(v));
            }
        }
        Self {
            cli,
            env: std::env::var_os(ENV_DATA_DIR).filter(|v| !v.is_empty()),
            exe_dir: std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf)),
            platform_config_dir: dirs::config_dir(),
        }
    }
}

/// Picks the data folder. Does not create it.
pub fn resolve(input: &ResolveInput) -> Option<DataDir> {
    if let Some(p) = &input.cli {
        return Some(DataDir {
            path: p.clone(),
            source: DataDirSource::CommandLine,
        });
    }
    if let Some(p) = &input.env {
        return Some(DataDir {
            path: PathBuf::from(p),
            source: DataDirSource::Environment,
        });
    }
    if let Some(exe) = &input.exe_dir {
        if exe.join(PORTABLE_FLAG).is_file() {
            return Some(DataDir {
                path: exe.join(PORTABLE_DATA_DIR),
                source: DataDirSource::Portable,
            });
        }
    }
    input.platform_config_dir.as_ref().map(|base| DataDir {
        path: base.join(APP_DIR_NAME),
        source: DataDirSource::Default,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(tmp: &Path) -> ResolveInput {
        ResolveInput {
            cli: None,
            env: None,
            exe_dir: Some(tmp.join("exe")),
            platform_config_dir: Some(tmp.join("appdata")),
        }
    }

    #[test]
    fn default_is_platform_config_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let d = resolve(&input(tmp.path())).unwrap();
        assert_eq!(d.source, DataDirSource::Default);
        assert_eq!(d.path, tmp.path().join("appdata").join(APP_DIR_NAME));
    }

    #[test]
    fn portable_flag_next_to_exe() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("exe")).unwrap();
        std::fs::write(tmp.path().join("exe").join(PORTABLE_FLAG), "").unwrap();
        let d = resolve(&input(tmp.path())).unwrap();
        assert_eq!(d.source, DataDirSource::Portable);
        assert_eq!(d.path, tmp.path().join("exe").join("data"));
    }

    #[test]
    fn env_beats_portable_and_cli_beats_env() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("exe")).unwrap();
        std::fs::write(tmp.path().join("exe").join(PORTABLE_FLAG), "").unwrap();
        let mut i = input(tmp.path());
        i.env = Some("E:/lps".into());
        assert_eq!(resolve(&i).unwrap().source, DataDirSource::Environment);
        i.cli = Some("F:/lps".into());
        let d = resolve(&i).unwrap();
        assert_eq!(d.source, DataDirSource::CommandLine);
        assert_eq!(d.path, PathBuf::from("F:/lps"));
    }
}
