//! Modal dialogs + floating UI: Environments manager, Save-draft
//! folder picker (with new-folder inline creator), cURL paste dialog,
//! right-side code snippet panel, toast notifications. Plus a couple
//! of state helpers tightly coupled to the save-draft modal
//! (folder-path lookup, subtree search) and `new_draft_request`.

use crate::env_compare::{compare_environments, display_value, safe_summary, EnvDiffEntry};
use crate::io::curl;
use crate::model::*;
use crate::snippet::{
    build_snippet_layout_job_content_only, render_snippet, render_snippet_redacted, SnippetLang,
};
use crate::theme::*;
use crate::widgets::*;
use crate::{
    backup, in_app_update_supported, open_update_log_in_os, runner, runner_detail, update_log_path,
    ApiClient, ExportDecision, RunnerResultRow,
};
use eframe::egui;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

mod about;
mod backup_modal;
mod env;
mod menu;
mod palette;
mod paste;
mod runner_modal;
mod save_draft;
mod snippet;
mod toast;
mod update;
