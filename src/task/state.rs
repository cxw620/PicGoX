//! State Model

#[derive(Debug, Clone, Copy)]
/// State of the uploader page
pub(crate) enum UploadState {
    /// Idle
    Idle,

    /// Uploading
    Uploading { progress: f32 },

    /// Cancelling
    Cancelling,
}

#[derive(Debug, Clone, Copy)]
/// State of the window
pub(crate) enum WindowState {
    /// Window is shown
    Shown,

    /// Window is hidden
    Hidden,

    /// Window is closed
    Closed,
}
