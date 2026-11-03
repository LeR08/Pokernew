//! Configuration layer of Local Poker Suite.
//!
//! * [`paths`] — where the data folder is (default, env, CLI, portable mode);
//! * [`defaults`] — embedded default files, installed on first run;
//! * [`family`] — the kinds of config files and their schemas;
//! * [`store`] — loading, JSON Schema validation, saving (comments kept);
//! * [`watch`] — hot reload on file changes.
//!
//! This crate knows nothing about Tauri, so it is tested on its own.

pub mod defaults;
pub mod family;
pub mod paths;
pub mod store;
pub mod tomljson;
pub mod watch;

pub use store::{ConfigEvent, ConfigStore, Issue};

/// Errors of the config layer.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("unknown config family '{0}'")]
    UnknownFamily(String),
    #[error("invalid config id '{0}'")]
    BadId(String),
    #[error("embedded schema '{0}' is missing")]
    MissingSchema(String),
    #[error("embedded schema '{0}' is invalid: {1}")]
    BadSchema(String, String),
    #[error("invalid config ({} issue(s))", .0.len())]
    Invalid(Vec<Issue>),
    #[error("file watcher: {0}")]
    Watch(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
