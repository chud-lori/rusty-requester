use crate::model::Folder;

pub(crate) struct InFlightSync {
    pub(crate) label: String,
    pub(crate) rx: std::sync::mpsc::Receiver<Result<SyncApply, String>>,
}

pub(crate) enum SyncApply {
    Toast(String),
    ReplaceFolders {
        folders: Vec<Folder>,
        message: String,
    },
    RefreshFolders {
        folders: Vec<Folder>,
        updated: usize,
    },
}

pub(crate) fn spawn<F>(label: impl Into<String>, job: F) -> InFlightSync
where
    F: FnOnce() -> Result<SyncApply, String> + Send + 'static,
{
    let label = label.into();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(job());
    });
    InFlightSync { label, rx }
}
