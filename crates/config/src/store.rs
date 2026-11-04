//! In-memory config store: loads every family from the data folder,
//! validates each file against its JSON Schema and keeps the last valid
//! version of each file. Invalid files never replace a valid one.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::Serialize;
use serde_json::Value;
use toml_edit::DocumentMut;

use crate::family::{classify, family, Family, FAMILIES};
use crate::tomljson::{merge_json_into_doc, toml_to_json};
use crate::{defaults, ConfigError};

/// One problem found in a config file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub family: String,
    pub id: String,
    /// JSON pointer to the offending value (`""` = whole file / syntax error).
    pub pointer: String,
    pub message: String,
}

/// What happened when a file was (re)loaded.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigEvent {
    /// A file was added or its content changed and is valid.
    Changed { family: String, id: String },
    /// A file was deleted.
    Removed { family: String, id: String },
    /// A file is invalid; the previous valid version (if any) is kept.
    Invalid {
        family: String,
        id: String,
        issues: Vec<Issue>,
    },
}

struct Entry {
    value: Value,
}

/// Thread-safe config store rooted at the data folder.
pub struct ConfigStore {
    root: PathBuf,
    validators: HashMap<&'static str, (Value, jsonschema::Validator)>,
    entries: RwLock<HashMap<(String, String), Entry>>,
    issues: RwLock<BTreeMap<(String, String), Vec<Issue>>>,
}

impl ConfigStore {
    /// Compiles the embedded schemas and loads every config file under `root`.
    /// Returns the store and the events of the initial load (invalid files).
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ConfigError> {
        let mut validators = HashMap::new();
        for fam in FAMILIES {
            let text = defaults::schema_text(fam.schema)
                .ok_or_else(|| ConfigError::MissingSchema(fam.schema.to_string()))?;
            let schema: Value = serde_json::from_str(text)
                .map_err(|e| ConfigError::BadSchema(fam.schema.to_string(), e.to_string()))?;
            let v = jsonschema::validator_for(&schema)
                .map_err(|e| ConfigError::BadSchema(fam.schema.to_string(), e.to_string()))?;
            validators.insert(fam.schema, (schema, v));
        }
        let store = Self {
            root: root.into(),
            validators,
            entries: RwLock::default(),
            issues: RwLock::default(),
        };
        for fam in FAMILIES {
            for id in fam.list_ids(&store.root) {
                if let Some(rel) = fam.rel_path(&id) {
                    store.reload_path(&store.root.join(rel));
                }
            }
        }
        Ok(store)
    }

    /// The data folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// JSON Schema of a family.
    pub fn schema(&self, family_name: &str) -> Option<Value> {
        let fam = family(family_name)?;
        self.validators.get(fam.schema).map(|(s, _)| s.clone())
    }

    /// Last valid value of a file.
    pub fn get(&self, family_name: &str, id: &str) -> Option<Value> {
        self.entries
            .read()
            .ok()?
            .get(&(family_name.to_string(), id.to_string()))
            .map(|e| e.value.clone())
    }

    /// Ids of the valid files of a family, sorted.
    pub fn list(&self, family_name: &str) -> Vec<String> {
        let Ok(entries) = self.entries.read() else {
            return vec![];
        };
        let mut ids: Vec<String> = entries
            .keys()
            .filter(|(f, _)| f == family_name)
            .map(|(_, id)| id.clone())
            .collect();
        ids.sort();
        ids
    }

    /// Current problems, one list per invalid file.
    pub fn issues(&self) -> Vec<Issue> {
        self.issues
            .read()
            .map(|m| m.values().flatten().cloned().collect())
            .unwrap_or_default()
    }

    /// Raw text of a file on disk.
    pub fn raw_text(&self, family_name: &str, id: &str) -> Result<String, ConfigError> {
        let path = self.path_of(family_name, id)?;
        Ok(std::fs::read_to_string(path)?)
    }

    /// Absolute path of a file (it may not exist yet).
    pub fn path_of(&self, family_name: &str, id: &str) -> Result<PathBuf, ConfigError> {
        let fam =
            family(family_name).ok_or_else(|| ConfigError::UnknownFamily(family_name.into()))?;
        let rel = fam
            .rel_path(id)
            .ok_or_else(|| ConfigError::BadId(id.into()))?;
        Ok(self.root.join(rel))
    }

    /// Parses and validates TOML text for a family, without saving.
    pub fn validate_text(
        &self,
        family_name: &str,
        id: &str,
        text: &str,
    ) -> Result<Value, Vec<Issue>> {
        let issue = |pointer: String, message: String| Issue {
            family: family_name.into(),
            id: id.into(),
            pointer,
            message,
        };
        let Some(fam) = family(family_name) else {
            return Err(vec![issue(
                String::new(),
                format!("unknown config family '{family_name}'"),
            )]);
        };
        let value = toml_to_json(text).map_err(|m| vec![issue(String::new(), m)])?;
        self.validate_value(fam, id, &value)?;
        Ok(value)
    }

    fn validate_value(&self, fam: &Family, id: &str, value: &Value) -> Result<(), Vec<Issue>> {
        let (_, validator) = &self.validators[fam.schema];
        let issues: Vec<Issue> = validator
            .iter_errors(value)
            .map(|e| Issue {
                family: fam.name.into(),
                id: id.into(),
                pointer: e.instance_path().to_string(),
                message: e.to_string(),
            })
            .collect();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    /// Validates then atomically writes raw TOML text, and updates memory.
    pub fn save_text(
        &self,
        family_name: &str,
        id: &str,
        text: &str,
    ) -> Result<ConfigEvent, ConfigError> {
        let value = self
            .validate_text(family_name, id, text)
            .map_err(ConfigError::Invalid)?;
        let path = self.path_of(family_name, id)?;
        write_atomic(&path, text)?;
        Ok(self.apply(family_name, id, value))
    }

    /// Validates a JSON value (from a settings form) and merges it into the
    /// existing TOML file, keeping comments and formatting.
    pub fn save_value(
        &self,
        family_name: &str,
        id: &str,
        value: &Value,
    ) -> Result<ConfigEvent, ConfigError> {
        let fam =
            family(family_name).ok_or_else(|| ConfigError::UnknownFamily(family_name.into()))?;
        let path = self.path_of(family_name, id)?;
        let mut doc: DocumentMut = match std::fs::read_to_string(&path) {
            Ok(text) => text.parse().unwrap_or_default(),
            Err(_) => DocumentMut::new(),
        };
        merge_json_into_doc(&mut doc, value).map_err(|m| {
            ConfigError::Invalid(vec![Issue {
                family: family_name.into(),
                id: id.into(),
                pointer: String::new(),
                message: m,
            }])
        })?;
        let text = doc.to_string();
        // Validate what will actually be on disk (number types may differ from the JSON).
        let on_disk = toml_to_json(&text).map_err(|m| {
            ConfigError::Invalid(vec![Issue {
                family: family_name.into(),
                id: id.into(),
                pointer: String::new(),
                message: m,
            }])
        })?;
        self.validate_value(fam, id, &on_disk)
            .map_err(ConfigError::Invalid)?;
        write_atomic(&path, &text)?;
        Ok(self.apply(family_name, id, on_disk))
    }

    /// Deletes a file of a folder family.
    pub fn delete(&self, family_name: &str, id: &str) -> Result<ConfigEvent, ConfigError> {
        let path = self.path_of(family_name, id)?;
        std::fs::remove_file(&path)?;
        Ok(self.forget(family_name, id))
    }

    /// Re-reads one file after a change on disk. Returns `None` when the path
    /// is not a config file or nothing changed.
    pub fn reload_path(&self, path: &Path) -> Option<ConfigEvent> {
        let (fam, id) = classify(&self.root, path)?;
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) if !path.exists() => return Some(self.forget(fam.name, &id)),
            Err(e) => {
                let issues = vec![Issue {
                    family: fam.name.into(),
                    id: id.clone(),
                    pointer: String::new(),
                    message: e.to_string(),
                }];
                return Some(self.set_issues(fam.name, &id, issues));
            }
        };
        match self.validate_text(fam.name, &id, &text) {
            Ok(value) => {
                let unchanged = self.get(fam.name, &id).as_ref() == Some(&value)
                    && !self.has_issues(fam.name, &id);
                if unchanged {
                    None
                } else {
                    Some(self.apply(fam.name, &id, value))
                }
            }
            Err(issues) => Some(self.set_issues(fam.name, &id, issues)),
        }
    }

    fn has_issues(&self, fam: &str, id: &str) -> bool {
        self.issues
            .read()
            .is_ok_and(|m| m.contains_key(&(fam.to_string(), id.to_string())))
    }

    fn apply(&self, fam: &str, id: &str, value: Value) -> ConfigEvent {
        let key = (fam.to_string(), id.to_string());
        if let Ok(mut m) = self.issues.write() {
            m.remove(&key);
        }
        if let Ok(mut m) = self.entries.write() {
            m.insert(key, Entry { value });
        }
        ConfigEvent::Changed {
            family: fam.into(),
            id: id.into(),
        }
    }

    fn forget(&self, fam: &str, id: &str) -> ConfigEvent {
        let key = (fam.to_string(), id.to_string());
        if let Ok(mut m) = self.issues.write() {
            m.remove(&key);
        }
        if let Ok(mut m) = self.entries.write() {
            m.remove(&key);
        }
        ConfigEvent::Removed {
            family: fam.into(),
            id: id.into(),
        }
    }

    fn set_issues(&self, fam: &str, id: &str, issues: Vec<Issue>) -> ConfigEvent {
        if let Ok(mut m) = self.issues.write() {
            m.insert((fam.to_string(), id.to_string()), issues.clone());
        }
        ConfigEvent::Invalid {
            family: fam.into(),
            id: id.into(),
            issues,
        }
    }
}

/// Writes `text` to a temporary sibling file, then renames it over `path`,
/// so readers (and the file watcher) never see a half-written file.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fresh() -> (tempfile::TempDir, ConfigStore) {
        let tmp = tempfile::tempdir().unwrap();
        defaults::install(tmp.path()).unwrap();
        let store = ConfigStore::open(tmp.path()).unwrap();
        (tmp, store)
    }

    #[test]
    fn every_default_file_is_valid() {
        let (_tmp, store) = fresh();
        assert!(store.issues().is_empty(), "{:#?}", store.issues());
        assert_eq!(
            store.list("themes"),
            vec![
                "bordeaux", "custom", "dark", "feutre", "graphite", "light", "ocean", "sable",
                "violet"
            ]
        );
        assert_eq!(store.list("profiles"), vec!["cash-6max", "mtt", "spin"]);
        assert_eq!(
            store.get("settings", "settings").unwrap()["general"]["language"],
            "fr"
        );
        // Tables keep the file order (stats are shown in the order they are written).
        let stats = store.get("stats", "core").unwrap();
        let ids: Vec<&String> = stats["stat"].as_object().unwrap().keys().collect();
        assert_eq!(ids[..3], ["bb100", "vpip", "pfr"]);
    }

    #[test]
    fn invalid_file_keeps_previous_value() {
        let (tmp, store) = fresh();
        let path = tmp.path().join("settings.toml");
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace("language = \"fr\"", "language = \"de\"");
        std::fs::write(&path, text).unwrap();
        match store.reload_path(&path) {
            Some(ConfigEvent::Invalid { issues, .. }) => {
                assert_eq!(issues[0].pointer, "/general/language");
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
        assert_eq!(
            store.get("settings", "settings").unwrap()["general"]["language"],
            "fr"
        );
        assert_eq!(store.issues().len(), 1);
    }

    #[test]
    fn syntax_error_is_reported() {
        let (_tmp, store) = fresh();
        let err = store.validate_text("themes", "x", "name = ").unwrap_err();
        assert_eq!(err[0].pointer, "");
    }

    #[test]
    fn save_value_round_trips_and_keeps_comments() {
        let (tmp, store) = fresh();
        let mut v = store.get("settings", "settings").unwrap();
        v["general"]["language"] = json!("en");
        v["appearance"]["font_scale"] = json!(1.2);
        store.save_value("settings", "settings", &v).unwrap();
        let text = std::fs::read_to_string(tmp.path().join("settings.toml")).unwrap();
        assert!(text.contains("# Langue de l'interface"));
        assert!(text.contains("language = \"en\""));
        assert_eq!(
            store.get("settings", "settings").unwrap()["general"]["language"],
            "en"
        );
        // Re-reading the same file reports no change.
        assert!(store
            .reload_path(&tmp.path().join("settings.toml"))
            .is_none());
    }

    #[test]
    fn save_value_rejects_invalid() {
        let (_tmp, store) = fresh();
        let mut v = store.get("settings", "settings").unwrap();
        v["appearance"]["density"] = json!("huge");
        assert!(matches!(
            store.save_value("settings", "settings", &v),
            Err(ConfigError::Invalid(_))
        ));
    }

    #[test]
    fn new_theme_file_then_delete() {
        let (tmp, store) = fresh();
        let text = defaults::default_text("themes/dark.toml")
            .unwrap()
            .replace("Sombre / Dark", "Mine");
        store.save_text("themes", "mine", &text).unwrap();
        assert!(tmp.path().join("themes/mine.toml").is_file());
        assert_eq!(store.get("themes", "mine").unwrap()["name"], "Mine");
        store.delete("themes", "mine").unwrap();
        assert!(store.get("themes", "mine").is_none());
    }
}
