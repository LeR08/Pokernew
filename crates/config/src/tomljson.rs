//! TOML <-> JSON bridges.
//!
//! Config files are TOML on disk but validated (JSON Schema) and served to the
//! UI as JSON. When the UI saves a form, the new JSON is *merged* into the
//! existing TOML document with `toml_edit`, so the user's comments and key
//! order survive; only values that actually changed are rewritten.

use serde_json::{Map, Value};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table};

/// Parses TOML text into a JSON value.
pub fn toml_to_json(text: &str) -> Result<Value, String> {
    toml::from_str::<Value>(text).map_err(|e| e.to_string())
}

/// Merges `value` (a JSON object) into `doc`: changed keys are updated in
/// place, new keys are appended, keys missing from `value` are removed.
pub fn merge_json_into_doc(doc: &mut DocumentMut, value: &Value) -> Result<(), String> {
    let obj = value.as_object().ok_or("top-level value must be an object")?;
    merge_table(doc.as_table_mut(), obj);
    Ok(())
}

fn merge_table(table: &mut Table, obj: &Map<String, Value>) {
    let stale: Vec<String> = table
        .iter()
        .map(|(k, _)| k.to_string())
        .filter(|k| !obj.get(k).is_some_and(|v| !v.is_null()))
        .collect();
    for k in stale {
        table.remove(&k);
    }
    for (k, v) in obj {
        if v.is_null() {
            continue;
        }
        match table.get_mut(k) {
            Some(item) => merge_item(item, v),
            None => {
                table.insert(k, new_item(v));
            }
        }
    }
}

fn merge_item(item: &mut Item, v: &Value) {
    match (&mut *item, v) {
        (Item::Table(t), Value::Object(o)) => merge_table(t, o),
        (Item::Value(old), v) => {
            if value_equals(old, v) {
                return;
            }
            let decor = old.decor().clone();
            let mut new = to_value(v);
            *new.decor_mut() = decor;
            *old = new;
        }
        _ => *item = new_item(v),
    }
}

/// True when an existing TOML value already holds `v` (avoids reformatting).
fn value_equals(old: &toml_edit::Value, v: &Value) -> bool {
    let mut probe = old.clone();
    probe.decor_mut().clear();
    toml_to_json(&format!("x = {probe}"))
        .ok()
        .and_then(|j| j.get("x").cloned())
        .is_some_and(|j| json_num_eq(&j, v))
}

/// JSON equality where `1` and `1.0` are equal (JS cannot tell them apart).
fn json_num_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| json_num_eq(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, a)| y.get(k).is_some_and(|b| json_num_eq(a, b)))
        }
        _ => a == b,
    }
}

fn new_item(v: &Value) -> Item {
    match v {
        Value::Object(o) => {
            let mut t = Table::new();
            merge_table(&mut t, o);
            Item::Table(t)
        }
        Value::Array(a) if !a.is_empty() && a.iter().all(Value::is_object) => {
            let mut aot = ArrayOfTables::new();
            for o in a.iter().filter_map(Value::as_object) {
                let mut t = Table::new();
                merge_table(&mut t, o);
                aot.push(t);
            }
            Item::ArrayOfTables(aot)
        }
        _ => Item::Value(to_value(v)),
    }
}

fn to_value(v: &Value) -> toml_edit::Value {
    match v {
        Value::Bool(b) => (*b).into(),
        Value::Number(n) => match n.as_i64() {
            Some(i) => i.into(),
            None => n.as_f64().unwrap_or(0.0).into(),
        },
        Value::String(s) => s.as_str().into(),
        Value::Array(a) => {
            let mut arr = Array::new();
            for x in a.iter().filter(|x| !x.is_null()) {
                arr.push(to_value(x));
            }
            toml_edit::Value::Array(arr)
        }
        Value::Object(o) => {
            let mut t = InlineTable::new();
            for (k, x) in o.iter().filter(|(_, x)| !x.is_null()) {
                t.insert(k, to_value(x));
            }
            toml_edit::Value::InlineTable(t)
        }
        // Nulls are filtered out by callers; fall back to an empty string.
        Value::Null => "".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SRC: &str = r#"# header comment
[general]
# the language
language = "fr" # trailing
profile = "mtt"

[modules]
order = ["a", "b"]
"#;

    #[test]
    fn merge_keeps_comments_and_updates_values() {
        let mut doc: DocumentMut = SRC.parse().unwrap();
        let mut v = toml_to_json(SRC).unwrap();
        v["general"]["language"] = json!("en");
        v["appearance"] = json!({ "font_scale": 1.1 });
        merge_json_into_doc(&mut doc, &v).unwrap();
        let out = doc.to_string();
        assert!(out.contains("# header comment"));
        assert!(out.contains("# the language"));
        assert!(out.contains("language = \"en\" # trailing"));
        assert!(out.contains("[appearance]"));
        assert_eq!(toml_to_json(&out).unwrap(), v);
    }

    #[test]
    fn unchanged_values_are_not_reformatted() {
        let src = "x = 1.0\narr = [\n  \"a\",\n  \"b\",\n]\n";
        let mut doc: DocumentMut = src.parse().unwrap();
        // JS sends 1.0 back as 1.
        merge_json_into_doc(&mut doc, &json!({ "x": 1, "arr": ["a", "b"] })).unwrap();
        assert_eq!(doc.to_string(), src);
    }

    #[test]
    fn removed_keys_disappear() {
        let mut doc: DocumentMut = SRC.parse().unwrap();
        merge_json_into_doc(&mut doc, &json!({ "general": { "language": "fr" } })).unwrap();
        let v = toml_to_json(&doc.to_string()).unwrap();
        assert_eq!(v, json!({ "general": { "language": "fr" } }));
    }
}
