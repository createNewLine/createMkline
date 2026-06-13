use iced::window::raw_window_handle::{HasWindowHandle, RawWindowHandle, WindowHandle};

/// Wraps RawWindowHandle to implement HasWindowHandle for rfd::set_parent.
/// RawWindowHandle is not Send on non-Windows platforms, but on Windows
/// (the only target) the active variant is always Win32 which is Send.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParentHandle(pub(crate) RawWindowHandle);

unsafe impl Send for ParentHandle {}
unsafe impl Sync for ParentHandle {}

impl HasWindowHandle for ParentHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, iced::window::raw_window_handle::HandleError> {
        Ok(unsafe { WindowHandle::borrow_raw(self.0) })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    // Source rows
    AddSource,
    RemoveSource(usize),
    SourcePath(usize, String),
    BrowseSourceDir(usize),
    BrowseSourceWithParent(usize, ParentHandle),
    SourceDirsPicked(usize, Vec<String>),

    // Target
    TargetPath(String),
    BrowseTargetDir,
    BrowseTargetWithParent(ParentHandle),
    TargetDirPicked(Option<String>),

    // Buttons
    Confirm,
    Cancel,
    BackupAll,
    ClearAll,

    // Async outcomes
    ConfirmResult(Result<String, String>),
    BackupResult(Result<String, String>),
    Noop,
}
