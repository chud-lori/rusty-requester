mod apply;
mod git;
pub(crate) mod job;

use crate::sync::job::SyncApply;
use crate::{io, ApiClient};
use std::path::PathBuf;

pub(crate) use job::InFlightSync;

impl ApiClient {
    pub(crate) fn choose_git_workspace_dir(&mut self) {
        if !self.state.settings.workspace_sync_enabled {
            self.open_sync_or_settings();
            return;
        }
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.state.sync.git_workspace_dir = path.display().to_string();
            self.save_state();
        }
    }

    pub(crate) fn choose_openapi_spec_file(&mut self) {
        if !self.state.settings.workspace_sync_enabled {
            self.open_sync_or_settings();
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenAPI", &["json", "yaml", "yml"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.state.sync.openapi_spec_path = path.display().to_string();
            self.save_state();
        }
    }

    pub(crate) fn import_git_workspace_from_config(&mut self) {
        if !self.state.settings.workspace_sync_enabled {
            self.open_sync_or_settings();
            return;
        }
        let root = self.state.sync.git_workspace_dir.trim();
        if root.is_empty() {
            self.show_toast("Choose a Git workspace directory first");
            return;
        }
        let root = PathBuf::from(root);
        self.spawn_sync_job("Importing Git workspace", move || {
            let folders = io::git_workspace::import_workspace_from_dir(&root)?;
            let n = folders.len();
            Ok(SyncApply::ReplaceFolders {
                folders,
                message: format!("Imported Git workspace ({} collection(s))", n),
            })
        });
    }

    pub(crate) fn export_git_workspace_from_config(&mut self) {
        if !self.state.settings.workspace_sync_enabled {
            self.open_sync_or_settings();
            return;
        }
        let root = self.state.sync.git_workspace_dir.trim();
        if root.is_empty() {
            self.show_toast("Choose a Git workspace directory first");
            return;
        }
        let root = PathBuf::from(root);
        let options = self.git_workspace_export_options();
        let folders = self.state.folders.clone();
        let include_secrets = self.state.sync.include_secrets_in_git_workspace;
        self.spawn_sync_job("Exporting Git workspace", move || {
            let summary = io::git_workspace::export_workspace_to_dir(&folders, &root, options)?;
            let mode = if include_secrets {
                "with secrets"
            } else {
                "masked"
            };
            Ok(SyncApply::Toast(format!(
                "Exported Git workspace {} ({} request file(s))",
                mode, summary.request_files
            )))
        });
    }

    pub(crate) fn refresh_openapi_from_config(&mut self) {
        const MAX_OPENAPI_REFRESH_BYTES: u64 = 10 * 1024 * 1024;

        if !self.state.settings.workspace_sync_enabled {
            self.open_sync_or_settings();
            return;
        }
        let path = self.state.sync.openapi_spec_path.trim();
        if path.is_empty() {
            self.show_toast("Choose an OpenAPI spec file first");
            return;
        }
        let path = PathBuf::from(path);
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) => {
                self.show_toast(format!("OpenAPI refresh failed: {}", e));
                return;
            }
        };
        if meta.file_type().is_symlink() {
            self.show_toast("OpenAPI refresh failed: symlink specs are not allowed");
            return;
        }
        if meta.len() > MAX_OPENAPI_REFRESH_BYTES {
            self.show_toast("OpenAPI refresh failed: spec is larger than 10 MB");
            return;
        }
        let mut refreshed_folders = self.state.folders.clone();
        self.spawn_sync_job("Refreshing OpenAPI requests", move || {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("OpenAPI read failed: {}", e))?;
            let updated = io::refresh_openapi_folders(&mut refreshed_folders, &content)?;
            if updated == 0 {
                Ok(SyncApply::Toast(
                    "No OpenAPI-generated requests matched this spec".to_string(),
                ))
            } else {
                Ok(SyncApply::RefreshFolders {
                    folders: refreshed_folders,
                    updated,
                })
            }
        });
    }

    pub(crate) fn pull_github_workspace_from_config(&mut self) {
        if !self.state.settings.workspace_sync_enabled {
            self.open_sync_or_settings();
            return;
        }
        let Some(root) = self.sync_git_root() else {
            return;
        };
        self.spawn_sync_job("Pulling GitHub workspace", move || {
            git::run(&root, &["pull", "--ff-only"]).map_err(|e| e.to_string())?;
            let folders = io::git_workspace::import_workspace_from_dir(&root)?;
            let n = folders.len();
            Ok(SyncApply::ReplaceFolders {
                folders,
                message: format!("Pulled Git workspace ({} collection(s))", n),
            })
        });
    }

    pub(crate) fn push_github_workspace_from_config(&mut self) {
        if !self.state.settings.workspace_sync_enabled {
            self.open_sync_or_settings();
            return;
        }
        let Some(root) = self.sync_git_root() else {
            return;
        };
        let folders = self.state.folders.clone();
        let options = self.git_workspace_export_options();
        let message = self.state.sync.git_commit_message.trim();
        let message = if message.is_empty() {
            "Sync Rusty Requester workspace"
        } else {
            message
        }
        .to_string();
        self.spawn_sync_job("Pushing GitHub workspace", move || {
            io::git_workspace::export_workspace_to_dir(&folders, &root, options)?;
            git::run(&root, &["add", "workspace.json", "requests"]).map_err(|e| e.to_string())?;
            match git::run(&root, &["commit", "-m", &message]) {
                Ok(_) | Err(git::GitError::NothingToCommit) => {}
                Err(e) => return Err(e.to_string()),
            }
            git::run(&root, &["push"]).map_err(|e| e.to_string())?;
            Ok(SyncApply::Toast(
                "Pushed workspace to Git remote".to_string(),
            ))
        });
    }

    fn git_workspace_export_options(&self) -> io::git_workspace::ExportOptions {
        io::git_workspace::ExportOptions {
            secret_policy: if self.state.sync.include_secrets_in_git_workspace {
                io::git_workspace::SecretPolicy::Include
            } else {
                io::git_workspace::SecretPolicy::Mask
            },
        }
    }

    fn sync_git_root(&mut self) -> Option<PathBuf> {
        let root = self.state.sync.git_workspace_dir.trim();
        if root.is_empty() {
            self.show_toast("Choose a Git workspace directory first");
            return None;
        }
        let root = PathBuf::from(root);
        if !root.join(".git").is_dir() {
            self.show_toast("Choose a Git repository root containing .git");
            return None;
        }
        Some(root)
    }

    fn spawn_sync_job<F>(&mut self, label: impl Into<String>, job: F)
    where
        F: FnOnce() -> Result<SyncApply, String> + Send + 'static,
    {
        if self.sync_in_flight.is_some() {
            self.show_toast("Workspace Sync is already running");
            return;
        }
        self.sync_in_flight = Some(job::spawn(label, job));
    }
}
