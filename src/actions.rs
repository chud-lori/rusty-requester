//! Actions palette — the counterpart to `command_palette` (⌘P).
//! ⌘P finds requests; ⇧⌘P triggers app actions (toggle panel,
//! duplicate tab, clear history, etc.). Actions are a fixed enum
//! so they're strongly-typed at the call site, self-document their
//! labels + keywords, and can advertise keyboard shortcuts in the
//! palette row.

/// Every action the ⇧⌘P palette can dispatch. Kept small enough to
/// fit on one screen; add new entries as the app grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteAction {
    NewRequest,
    DuplicateTab,
    CloseTab,
    TogglePin,
    SaveDraft,
    CopyAsCurl,
    ToggleSnippetPanel,
    OpenEnvironments,
    OpenSettings,
    PasteCurl,
    ImportCollection,
    ExportJson,
    ExportYaml,
    ClearHistory,
    ToggleSidebarHistory,
    ShowAbout,
}

impl PaletteAction {
    /// Every action in palette order. Ordering matters — the first
    /// match for a given fuzzy query wins the initial selection.
    pub const ALL: &'static [Self] = &[
        Self::NewRequest,
        Self::DuplicateTab,
        Self::CloseTab,
        Self::TogglePin,
        Self::SaveDraft,
        Self::CopyAsCurl,
        Self::ToggleSnippetPanel,
        Self::OpenEnvironments,
        Self::OpenSettings,
        Self::PasteCurl,
        Self::ImportCollection,
        Self::ExportJson,
        Self::ExportYaml,
        Self::ClearHistory,
        Self::ToggleSidebarHistory,
        Self::ShowAbout,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::NewRequest => "New request",
            Self::DuplicateTab => "Duplicate tab",
            Self::CloseTab => "Close tab",
            Self::TogglePin => "Toggle pin tab",
            Self::SaveDraft => "Save draft to folder…",
            Self::CopyAsCurl => "Copy as cURL",
            Self::ToggleSnippetPanel => "Toggle code snippet panel",
            Self::OpenEnvironments => "Open environments…",
            Self::OpenSettings => "Open settings…",
            Self::PasteCurl => "Paste cURL command…",
            Self::ImportCollection => "Import collection file…",
            Self::ExportJson => "Export all as JSON…",
            Self::ExportYaml => "Export all as YAML…",
            Self::ClearHistory => "Clear history",
            Self::ToggleSidebarHistory => "Toggle sidebar History / Collections view",
            Self::ShowAbout => "About Rusty Requester",
        }
    }

    /// Extra search keywords — things the user might type that aren't
    /// in the label. Kept lowercase; the palette searches on
    /// `label + keywords`.
    pub fn keywords(&self) -> &'static str {
        match self {
            Self::NewRequest => "create add tab",
            Self::DuplicateTab => "clone copy tab",
            Self::CloseTab => "close window",
            Self::TogglePin => "pin unpin sticky",
            Self::SaveDraft => "save persist folder collection",
            Self::CopyAsCurl => "curl command line shell",
            Self::ToggleSnippetPanel => "code panel side python javascript fetch httpie",
            Self::OpenEnvironments => "env variables vars",
            Self::OpenSettings => "preferences timeout proxy tls",
            Self::PasteCurl => "curl import clipboard",
            Self::ImportCollection => "import postman json yaml",
            Self::ExportJson => "export backup json",
            Self::ExportYaml => "export backup yaml",
            Self::ClearHistory => "delete wipe log",
            Self::ToggleSidebarHistory => "switch toggle sidebar history collections",
            Self::ShowAbout => "about version credit help",
        }
    }

    /// Keyboard shortcut display, right-aligned in the palette row.
    /// `None` = no shortcut (rare, but some actions are palette-only).
    pub fn shortcut(&self) -> Option<&'static str> {
        match self {
            Self::NewRequest => Some("⌘N"),
            Self::DuplicateTab => Some("⌘D"),
            Self::CloseTab => Some("⌘W"),
            Self::SaveDraft => Some("⌘S"),
            Self::ToggleSnippetPanel => Some("⇧⌘C"),
            Self::OpenSettings => Some("⌘,"),
            _ => None,
        }
    }

    /// Haystack string used by the fuzzy matcher (lowercase,
    /// `label + " " + keywords`).
    pub fn haystack_lc(&self) -> String {
        format!("{} {}", self.label(), self.keywords()).to_lowercase()
    }
}
