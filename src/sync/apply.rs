use crate::find_folder_by_id_mut;
use crate::sync::job::SyncApply;
use crate::{backup, ApiClient};
use eframe::egui;

impl ApiClient {
    pub(crate) fn poll_sync_job(&mut self, ctx: &egui::Context) {
        if let Some(f) = &self.sync_in_flight {
            match f.rx.try_recv() {
                Ok(result) => {
                    self.sync_in_flight = None;
                    self.apply_sync_result(result);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint();
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.sync_in_flight = None;
                    self.show_toast("Workspace Sync stopped before returning a result");
                }
            }
        }
    }

    fn apply_sync_result(&mut self, result: Result<SyncApply, String>) {
        match result {
            Ok(SyncApply::Toast(message)) => self.show_toast(message),
            Ok(SyncApply::ReplaceWorkspace {
                folders,
                environments,
                message,
            }) => {
                if let Err(e) = backup::create_backup(&self.storage_path) {
                    self.show_toast(format!("Sync aborted: backup failed: {}", e));
                    return;
                }
                self.state.folders = folders;
                self.state.environments = environments;
                self.prune_stale_tabs();
                self.save_state();
                self.show_toast(message);
            }
            Ok(SyncApply::ReplaceCollection {
                folder_id,
                mut folder,
                environments,
                message,
            }) => {
                if let Err(e) = backup::create_backup(&self.storage_path) {
                    self.show_toast(format!("Sync aborted: backup failed: {}", e));
                    return;
                }
                if let Some(existing) = find_folder_by_id_mut(&mut self.state.folders, &folder_id) {
                    folder.sync = existing.sync.clone();
                    *existing = *folder;
                    if !environments.is_empty() {
                        self.state.environments = environments;
                    }
                    self.prune_stale_tabs();
                    self.save_state();
                    self.show_toast(message);
                } else {
                    self.show_toast("Sync aborted: collection no longer exists");
                }
            }
            Ok(SyncApply::CollectionGitStatus { status }) => {
                self.collection_git_status = status;
            }
            Ok(SyncApply::RefreshFolders { folders, updated }) => {
                if let Err(e) = backup::create_backup(&self.storage_path) {
                    self.show_toast(format!("Refresh aborted: backup failed: {}", e));
                    return;
                }
                self.state.folders = folders;
                self.save_state();
                self.load_request_for_editing();
                self.show_toast(format!("Refreshed {} OpenAPI request(s)", updated));
            }
            Err(e) => self.show_toast(format!("Workspace Sync failed: {}", e)),
        }
    }
}
