//! UI rendering — every `render_*` method on `ApiClient` lives in
//! one of these submodules, split by area of the screen they drive.
//! The `ApiClient` struct itself stays in `main.rs` (for now); these
//! files only add `impl ApiClient { ... }` blocks.

pub mod editor;
pub mod modals;
pub mod response;
pub mod sidebar;
