use crate::model::{Environment, KvRow};
use crate::privacy::{is_sensitive_key, mask_secret_value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvValue {
    pub value: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvDiffEntry {
    pub key: String,
    pub source: Option<EnvValue>,
    pub target: Option<EnvValue>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnvDiff {
    pub added: Vec<EnvDiffEntry>,
    pub missing: Vec<EnvDiffEntry>,
    pub changed: Vec<EnvDiffEntry>,
    pub unchanged: Vec<EnvDiffEntry>,
}

impl EnvDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.missing.is_empty()
            && self.changed.is_empty()
            && self.unchanged.is_empty()
    }
}

pub fn compare_environments(source: &Environment, target: &Environment) -> EnvDiff {
    compare_variable_rows(&source.variables, &target.variables)
}

fn compare_variable_rows(source: &[KvRow], target: &[KvRow]) -> EnvDiff {
    let source_map = keyed_values(source);
    let target_map = keyed_values(target);
    let keys: BTreeSet<&str> = source_map
        .keys()
        .chain(target_map.keys())
        .map(String::as_str)
        .collect();
    let mut diff = EnvDiff::default();

    for key in keys {
        let source = source_map.get(key).cloned();
        let target = target_map.get(key).cloned();
        let entry = EnvDiffEntry {
            key: key.to_string(),
            source,
            target,
        };
        match (&entry.source, &entry.target) {
            (Some(_), None) => diff.missing.push(entry),
            (None, Some(_)) => diff.added.push(entry),
            (Some(source), Some(target)) if source == target => diff.unchanged.push(entry),
            (Some(_), Some(_)) => diff.changed.push(entry),
            (None, None) => {}
        }
    }

    diff
}

pub fn display_value(key: &str, value: &str) -> String {
    if is_sensitive_key(key) {
        mask_secret_value(value)
    } else {
        value.to_string()
    }
}

pub fn safe_summary(source_name: &str, target_name: &str, diff: &EnvDiff) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Environment diff: {} -> {}", source_name, target_name);
    let _ = writeln!(
        out,
        "Added: {} | Missing: {} | Changed: {} | Unchanged: {}",
        diff.added.len(),
        diff.missing.len(),
        diff.changed.len(),
        diff.unchanged.len()
    );
    write_group(&mut out, "Added in target", &diff.added);
    write_group(&mut out, "Missing from target", &diff.missing);
    write_group(&mut out, "Changed", &diff.changed);
    write_group(&mut out, "Unchanged", &diff.unchanged);
    out
}

fn keyed_values(rows: &[KvRow]) -> BTreeMap<String, EnvValue> {
    let mut values = BTreeMap::new();
    for row in rows {
        let key = row.key.trim();
        if key.is_empty() {
            continue;
        }
        values.entry(key.to_string()).or_insert_with(|| EnvValue {
            value: row.value.clone(),
            enabled: row.enabled,
        });
    }
    values
}

fn write_group(out: &mut String, title: &str, entries: &[EnvDiffEntry]) {
    if entries.is_empty() {
        return;
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "{}:", title);
    for entry in entries {
        match (&entry.source, &entry.target) {
            (Some(source), Some(target)) => {
                let _ = writeln!(
                    out,
                    "- {}: {} -> {}{}",
                    entry.key,
                    display_value(&entry.key, &source.value),
                    display_value(&entry.key, &target.value),
                    enabled_note(source.enabled, target.enabled)
                );
            }
            (Some(source), None) => {
                let _ = writeln!(
                    out,
                    "- {}: {}{}",
                    entry.key,
                    display_value(&entry.key, &source.value),
                    if source.enabled { "" } else { " (disabled)" }
                );
            }
            (None, Some(target)) => {
                let _ = writeln!(
                    out,
                    "- {}: {}{}",
                    entry.key,
                    display_value(&entry.key, &target.value),
                    if target.enabled { "" } else { " (disabled)" }
                );
            }
            (None, None) => {}
        }
    }
}

fn enabled_note(source_enabled: bool, target_enabled: bool) -> String {
    if source_enabled == target_enabled {
        String::new()
    } else {
        format!(
            " ({} -> {})",
            if source_enabled {
                "enabled"
            } else {
                "disabled"
            },
            if target_enabled {
                "enabled"
            } else {
                "disabled"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str, rows: Vec<KvRow>) -> Environment {
        Environment {
            id: name.to_string(),
            name: name.to_string(),
            variables: rows,
            cookies: vec![],
        }
    }

    #[test]
    fn groups_added_missing_changed_and_unchanged_keys() {
        let source = env(
            "Local",
            vec![
                KvRow::new("API_URL", "http://localhost"),
                KvRow::new("TIMEOUT", "30"),
                KvRow::new("SOURCE_ONLY", "copy-me"),
            ],
        );
        let target = env(
            "Prod",
            vec![
                KvRow::new("API_URL", "https://api.example.com"),
                KvRow::new("TIMEOUT", "30"),
                KvRow::new("TARGET_ONLY", "already-there"),
            ],
        );

        let diff = compare_environments(&source, &target);

        assert_eq!(
            diff.changed
                .iter()
                .map(|e| e.key.as_str())
                .collect::<Vec<_>>(),
            vec!["API_URL"]
        );
        assert_eq!(
            diff.unchanged
                .iter()
                .map(|e| e.key.as_str())
                .collect::<Vec<_>>(),
            vec!["TIMEOUT"]
        );
        assert_eq!(
            diff.missing
                .iter()
                .map(|e| e.key.as_str())
                .collect::<Vec<_>>(),
            vec!["SOURCE_ONLY"]
        );
        assert_eq!(
            diff.added
                .iter()
                .map(|e| e.key.as_str())
                .collect::<Vec<_>>(),
            vec!["TARGET_ONLY"]
        );
    }

    #[test]
    fn masks_sensitive_values_in_display_and_summary() {
        let source = env("Local", vec![KvRow::new("api_token", "super-secret-token")]);
        let target = env("Prod", vec![KvRow::new("api_token", "different-secret")]);
        let diff = compare_environments(&source, &target);
        let summary = safe_summary("Local", "Prod", &diff);

        assert_eq!(
            display_value("api_token", "super-secret-token"),
            "super-...oken"
        );
        assert!(summary.contains("api_token: super-...oken -> differ...cret"));
        assert!(!summary.contains("super-secret-token"));
        assert!(!summary.contains("different-secret"));
    }

    #[test]
    fn enabled_state_changes_count_as_changed() {
        let mut source_row = KvRow::new("FEATURE", "on");
        source_row.enabled = true;
        let mut target_row = KvRow::new("FEATURE", "on");
        target_row.enabled = false;

        let diff = compare_environments(&env("A", vec![source_row]), &env("B", vec![target_row]));

        assert_eq!(diff.changed.len(), 1);
        assert!(safe_summary("A", "B", &diff).contains("(enabled -> disabled)"));
    }
}
