use crate::model::AppState;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BACKUP_DIR_NAME: &str = "backups";

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub created_at: Option<SystemTime>,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct RestoreOutcome {
    pub restored_from: PathBuf,
    pub current_backup: Option<PathBuf>,
}

pub fn backup_dir_for(storage_path: &Path) -> PathBuf {
    storage_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(BACKUP_DIR_NAME)
}

pub fn backup_path_for(storage_path: &Path, timestamp: SystemTime) -> PathBuf {
    let duration = timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    backup_path_for_parts(storage_path, duration.as_secs(), duration.subsec_nanos())
}

pub fn create_backup(storage_path: &Path) -> io::Result<Option<PathBuf>> {
    if !storage_path.exists() {
        return Ok(None);
    }

    let backup_dir = backup_dir_for(storage_path);
    fs::create_dir_all(&backup_dir)?;
    harden_dir_permissions(&backup_dir)?;

    let mut backup_path = backup_path_for(storage_path, SystemTime::now());
    backup_path = first_available_backup_path(&backup_path);

    fs::copy(storage_path, &backup_path)?;
    harden_file_permissions(&backup_path)?;
    Ok(Some(backup_path))
}

#[allow(dead_code)]
pub fn list_backups(storage_path: &Path) -> io::Result<Vec<BackupEntry>> {
    let backup_dir = backup_dir_for(storage_path);
    let entries = match fs::read_dir(&backup_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    let mut backups = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if is_symlink(&path)? || !path.is_file() || !is_backup_for(storage_path, &path) {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().into_owned();
        let metadata = entry.metadata()?;
        backups.push(BackupEntry {
            created_at: parse_backup_timestamp(storage_path, &file_name),
            file_name,
            path,
            size_bytes: metadata.len(),
        });
    }

    backups.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.file_name.cmp(&a.file_name))
    });
    Ok(backups)
}

#[allow(dead_code)]
pub fn restore_backup(storage_path: &Path, backup_path: &Path) -> io::Result<RestoreOutcome> {
    validate_restore_source(storage_path, backup_path)?;

    let bytes = fs::read(backup_path)?;
    serde_json::from_slice::<AppState>(&bytes).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("backup is not a valid workspace state: {}", e),
        )
    })?;

    if !backup_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "backup file does not exist",
        ));
    }

    let current_backup = create_backup(storage_path)?;
    atomic_write_into(&bytes, storage_path)?;

    Ok(RestoreOutcome {
        restored_from: backup_path.to_path_buf(),
        current_backup,
    })
}

fn validate_restore_source(storage_path: &Path, backup_path: &Path) -> io::Result<()> {
    if is_symlink(backup_path)? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "backup symlinks cannot be restored",
        ));
    }

    let backup_dir = backup_dir_for(storage_path);
    if !backup_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "backup must come from this workspace backup directory",
        ));
    }
    let backup_dir = backup_dir.canonicalize()?;
    let backup_path = backup_path.canonicalize()?;
    if backup_path.parent() != Some(backup_dir.as_path())
        || !is_backup_for(storage_path, &backup_path)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "backup must come from this workspace backup directory",
        ));
    }

    Ok(())
}

fn backup_path_for_parts(storage_path: &Path, unix_secs: u64, nanos: u32) -> PathBuf {
    let stem = backup_stem(storage_path);
    let extension = storage_path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("json");
    backup_dir_for(storage_path).join(format!("{}-{}-{}.{}", stem, unix_secs, nanos, extension))
}

fn backup_stem(storage_path: &Path) -> String {
    storage_path
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("data")
        .to_string()
}

fn first_available_backup_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or("data");
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("json");

    for n in 1.. {
        let candidate = parent.join(format!("{}-{}.{}", stem, n, extension));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded backup suffix search should always find a free path")
}

#[allow(dead_code)]
fn is_backup_for(storage_path: &Path, path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    parse_backup_timestamp(storage_path, file_name).is_some()
}

#[allow(dead_code)]
fn parse_backup_timestamp(storage_path: &Path, file_name: &str) -> Option<SystemTime> {
    let stem = backup_stem(storage_path);
    let extension = storage_path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("json");
    let prefix = format!("{}-", stem);
    let suffix = format!(".{}", extension);
    let inner = file_name.strip_prefix(&prefix)?.strip_suffix(&suffix)?;
    let (secs, nanos) = inner.split_once('-')?;
    let secs = secs.parse::<u64>().ok()?;
    let nanos = nanos.parse::<u32>().ok()?;
    if nanos >= 1_000_000_000 {
        return None;
    }
    UNIX_EPOCH.checked_add(Duration::new(secs, nanos))
}

#[allow(dead_code)]
fn atomic_write_into(bytes: &[u8], target: &Path) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
        harden_dir_permissions(parent)?;
    }

    let tmp = restore_tmp_path(target);
    let result = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        harden_file_permissions(&tmp)?;
        fs::rename(&tmp, target)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn is_symlink(path: &Path) -> io::Result<bool> {
    fs::symlink_metadata(path).map(|metadata| metadata.file_type().is_symlink())
}

#[cfg(unix)]
fn harden_dir_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn harden_dir_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn harden_file_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn harden_file_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[allow(dead_code)]
fn restore_tmp_path(target: &Path) -> PathBuf {
    let mut tmp = target.to_path_buf();
    let extension = target.extension().and_then(OsStr::to_str).unwrap_or("tmp");
    tmp.set_extension(format!("{}.restore.tmp", extension));
    tmp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_workspace(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("rusty-requester-backup-test-{}-{}", name, nonce));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn backup_path_lives_next_to_data_file_in_backups_dir() {
        let storage_path = PathBuf::from("/tmp/rusty-requester/data.json");
        let backup_path = backup_path_for(&storage_path, UNIX_EPOCH + Duration::new(42, 7));

        assert_eq!(
            backup_path,
            PathBuf::from("/tmp/rusty-requester/backups/data-42-7.json")
        );
    }

    #[test]
    fn create_backup_returns_none_when_storage_file_is_missing() {
        let root = temp_workspace("missing");
        let storage_path = root.join("data.json");

        let backup = create_backup(&storage_path).unwrap();

        assert_eq!(backup, None);
        assert!(!backup_dir_for(&storage_path).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_and_list_backups_for_storage_file() {
        let root = temp_workspace("list");
        let storage_path = root.join("data.json");
        fs::write(&storage_path, br#"{"folders":[]}"#).unwrap();

        let backup_path = create_backup(&storage_path).unwrap().unwrap();
        fs::write(
            backup_dir_for(&storage_path).join("unrelated.txt"),
            b"ignore",
        )
        .unwrap();

        let backups = list_backups(&storage_path).unwrap();

        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].path, backup_path);
        assert_eq!(backups[0].size_bytes, br#"{"folders":[]}"#.len() as u64);
        assert!(backups[0].created_at.is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_backup_preserves_current_state_first() {
        let root = temp_workspace("restore");
        let storage_path = root.join("data.json");
        fs::write(
            &storage_path,
            br#"{"folders":[{"id":"current","name":"Current","requests":[]}]}"#,
        )
        .unwrap();

        let backup_path = backup_dir_for(&storage_path).join("data-1-0.json");
        fs::create_dir_all(backup_dir_for(&storage_path)).unwrap();
        fs::write(
            &backup_path,
            br#"{"folders":[{"id":"restored","name":"Restored","requests":[]}]}"#,
        )
        .unwrap();

        let outcome = restore_backup(&storage_path, &backup_path).unwrap();

        assert_eq!(
            fs::read(&storage_path).unwrap(),
            br#"{"folders":[{"id":"restored","name":"Restored","requests":[]}]}"#
        );
        let current_backup = outcome.current_backup.unwrap();
        assert_eq!(
            fs::read(current_backup).unwrap(),
            br#"{"folders":[{"id":"current","name":"Current","requests":[]}]}"#
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_backup_rejects_arbitrary_files() {
        let root = temp_workspace("reject-arbitrary");
        let storage_path = root.join("data.json");
        fs::write(&storage_path, br#"{"folders":[]}"#).unwrap();
        let outside_backup = root.join("outside.json");
        fs::write(&outside_backup, br#"{"folders":[]}"#).unwrap();

        let err = restore_backup(&storage_path, &outside_backup).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_backup_rejects_invalid_workspace_json() {
        let root = temp_workspace("reject-invalid");
        let storage_path = root.join("data.json");
        fs::write(&storage_path, br#"{"folders":[]}"#).unwrap();
        let backup_path = backup_dir_for(&storage_path).join("data-1-0.json");
        fs::create_dir_all(backup_dir_for(&storage_path)).unwrap();
        fs::write(&backup_path, br#"{"folders":"not a folder list"}"#).unwrap();

        let err = restore_backup(&storage_path, &backup_path).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let _ = fs::remove_dir_all(root);
    }
}
