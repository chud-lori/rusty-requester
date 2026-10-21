//! Native OS credential-store backing for secret environment variables.
//!
//! A variable row flagged `secret` keeps its value in the macOS Keychain /
//! Linux Secret Service / Windows Credential Manager (via the `keyring`
//! crate) instead of `data.json`. The file keeps only the `secret` flag and
//! an opaque `secret_ref` account; the value itself never touches disk.
//!
//! Everything degrades to the plain in-file value when no credential store
//! is available, so the user's data is never lost and the app never crashes.

use crate::model::AppState;
use uuid::Uuid;

/// Credential-store namespace. Stable and app-specific so entries group
/// under this app in the OS viewer and never collide with other tools.
const SERVICE: &str = "com.chud-lori.rusty-requester";

/// Put, fetch, and remove secret strings keyed by an opaque account.
/// Abstracted so the persist / resolve logic can be exercised against an
/// in-memory or deliberately-unavailable store in tests without ever
/// touching the real OS keychain.
pub trait SecretStore {
    fn available(&self) -> bool;
    /// `Ok(None)` = no such entry; `Err` = backend failure.
    fn get(&self, account: &str) -> Result<Option<String>, ()>;
    fn set(&self, account: &str, value: &str) -> Result<(), ()>;
    fn delete(&self, account: &str) -> Result<(), ()>;
}

/// The real OS credential store via the `keyring` crate.
pub struct Keyring;

impl SecretStore for Keyring {
    fn available(&self) -> bool {
        // `store_status` initializes the platform store without reading or
        // writing any entry, so it never prompts the user.
        keyring::Entry::store_status().is_ok()
    }

    fn get(&self, account: &str) -> Result<Option<String>, ()> {
        let entry = keyring::Entry::new(SERVICE, account).map_err(|_| ())?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(()),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<(), ()> {
        keyring::Entry::new(SERVICE, account)
            .and_then(|e| e.set_password(value))
            .map_err(|_| ())
    }

    fn delete(&self, account: &str) -> Result<(), ()> {
        let entry = keyring::Entry::new(SERVICE, account).map_err(|_| ())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

/// True when any variable carries secret metadata, so the common
/// no-secrets save path can skip the clone + keychain work entirely.
pub fn has_secret_metadata(state: &AppState) -> bool {
    state
        .environments
        .iter()
        .flat_map(|e| &e.variables)
        .any(|v| v.secret || !v.secret_ref.is_empty())
}

/// Move secret values from `data.json` into the credential store on load.
pub fn resolve_secrets(state: &mut AppState) {
    resolve_secrets_with(&Keyring, state);
}

/// Sync secret values into the credential store and return the sanitized
/// state to serialize — externalized values are blanked so only a reference
/// reaches `data.json`.
pub fn persist_secrets(state: &mut AppState) -> AppState {
    persist_secrets_with(&Keyring, state)
}

/// Fill in-memory values from the store. A missing entry or a backend error
/// leaves whatever value the file held, so a degraded save that fell back to
/// the plain in-file value is never clobbered.
fn resolve_secrets_with<S: SecretStore>(store: &S, state: &mut AppState) {
    for env in &mut state.environments {
        for var in &mut env.variables {
            if !var.secret || var.secret_ref.is_empty() {
                continue;
            }
            if let Ok(Some(value)) = store.get(&var.secret_ref) {
                var.value = value;
            }
        }
    }
}

fn persist_secrets_with<S: SecretStore>(store: &S, state: &mut AppState) -> AppState {
    let available = store.available();
    let mut disk = state.clone();

    for (env, disk_env) in state
        .environments
        .iter_mut()
        .zip(disk.environments.iter_mut())
    {
        for (var, disk_var) in env.variables.iter_mut().zip(disk_env.variables.iter_mut()) {
            if var.secret {
                // Externalize only when a store is reachable and there is a
                // value to store. On success the reference stays in the file
                // and the value is blanked; otherwise the value stays in the
                // file as a fallback so nothing is lost.
                if available && !var.value.is_empty() {
                    if var.secret_ref.is_empty() {
                        let account = Uuid::new_v4().to_string();
                        var.secret_ref = account.clone();
                        disk_var.secret_ref = account;
                    }
                    if store.set(&var.secret_ref, &var.value).is_ok() {
                        disk_var.value.clear();
                    }
                }
            } else if !var.secret_ref.is_empty() {
                // Row was un-marked: drop its keychain entry and reference so
                // the value returns to the file.
                let _ = store.delete(&var.secret_ref);
                var.secret_ref.clear();
                disk_var.secret_ref.clear();
            }
        }
    }

    disk
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Environment, KvRow};
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// In-memory store that records every value set, to assert on the
    /// externalized reference and round-trip without a real keychain.
    struct FakeStore {
        available: bool,
        entries: RefCell<HashMap<String, String>>,
    }

    impl FakeStore {
        fn up() -> Self {
            Self {
                available: true,
                entries: RefCell::new(HashMap::new()),
            }
        }
        fn down() -> Self {
            Self {
                available: false,
                entries: RefCell::new(HashMap::new()),
            }
        }
    }

    impl SecretStore for FakeStore {
        fn available(&self) -> bool {
            self.available
        }
        fn get(&self, account: &str) -> Result<Option<String>, ()> {
            Ok(self.entries.borrow().get(account).cloned())
        }
        fn set(&self, account: &str, value: &str) -> Result<(), ()> {
            self.entries
                .borrow_mut()
                .insert(account.to_string(), value.to_string());
            Ok(())
        }
        fn delete(&self, account: &str) -> Result<(), ()> {
            self.entries.borrow_mut().remove(account);
            Ok(())
        }
    }

    fn state_with(var: KvRow) -> AppState {
        AppState {
            environments: vec![Environment {
                id: "env1".to_string(),
                name: "Env".to_string(),
                variables: vec![var],
                cookies: vec![],
            }],
            ..Default::default()
        }
    }

    fn secret_var() -> KvRow {
        KvRow {
            secret: true,
            ..KvRow::new("TOKEN", "s3cret")
        }
    }

    #[test]
    fn secret_value_is_not_serialized_only_the_reference() {
        let store = FakeStore::up();
        let mut state = state_with(secret_var());
        let disk = persist_secrets_with(&store, &mut state);

        let row = &disk.environments[0].variables[0];
        assert!(row.secret);
        assert!(!row.secret_ref.is_empty(), "a reference must be assigned");
        assert!(row.value.is_empty(), "the secret value must not persist");

        let json = serde_json::to_string(&disk).unwrap();
        assert!(!json.contains("s3cret"), "secret must not reach data.json");
        assert!(json.contains(&row.secret_ref));

        // The live state keeps the value so request-time consumers still work.
        assert_eq!(state.environments[0].variables[0].value, "s3cret");
        // The value landed in the store under the reference.
        assert_eq!(
            store.get(&row.secret_ref).unwrap().as_deref(),
            Some("s3cret")
        );
    }

    #[test]
    fn degrades_to_in_file_value_when_store_unavailable() {
        let store = FakeStore::down();
        let mut state = state_with(secret_var());
        let disk = persist_secrets_with(&store, &mut state);

        // No store: the value stays in the file so the user never loses it.
        assert_eq!(disk.environments[0].variables[0].value, "s3cret");
        assert_eq!(state.environments[0].variables[0].value, "s3cret");
    }

    #[test]
    fn resolve_fills_value_from_store() {
        let store = FakeStore::up();
        let mut state = state_with(secret_var());
        persist_secrets_with(&store, &mut state);
        let reference = state.environments[0].variables[0].secret_ref.clone();

        // Simulate a reload: the file had only the reference, value blank.
        let mut reloaded = state_with(KvRow {
            secret: true,
            secret_ref: reference,
            ..KvRow::new("TOKEN", "")
        });
        resolve_secrets_with(&store, &mut reloaded);
        assert_eq!(reloaded.environments[0].variables[0].value, "s3cret");
    }

    #[test]
    fn resolve_keeps_file_value_when_entry_missing() {
        let store = FakeStore::up();
        let mut reloaded = state_with(KvRow {
            secret: true,
            secret_ref: "orphan-ref".to_string(),
            ..KvRow::new("TOKEN", "fallback")
        });
        resolve_secrets_with(&store, &mut reloaded);
        // Entry absent from the store, but the file had a fallback value:
        // keep it rather than blanking the field.
        assert_eq!(reloaded.environments[0].variables[0].value, "fallback");
    }

    #[test]
    fn unmarking_secret_deletes_entry_and_reference() {
        let store = FakeStore::up();
        let mut state = state_with(secret_var());
        persist_secrets_with(&store, &mut state);
        let reference = state.environments[0].variables[0].secret_ref.clone();

        state.environments[0].variables[0].secret = false;
        let disk = persist_secrets_with(&store, &mut state);

        assert!(state.environments[0].variables[0].secret_ref.is_empty());
        assert_eq!(disk.environments[0].variables[0].value, "s3cret");
        assert_eq!(store.get(&reference).unwrap(), None);
    }

    #[test]
    fn old_data_json_without_secret_fields_loads_as_plain() {
        let json = r#"{"enabled":true,"key":"TOKEN","value":"plain"}"#;
        let row: KvRow = serde_json::from_str(json).unwrap();
        assert!(!row.secret);
        assert!(row.secret_ref.is_empty());
        assert_eq!(row.value, "plain");
    }
}
