use iced_futures::futures;
use sctk::reexports::client::ConnectError;

/// An error that occurred while running an application.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The futures executor could not be created.
    #[error("the futures executor could not be created")]
    ExecutorCreationFailed(futures::io::Error),

    /// The application window could not be created.
    #[error("the application window could not be created")]
    WindowCreationFailed(Box<dyn std::error::Error + Send + Sync>),

    /// The application graphics context could not be created.
    #[error("the application graphics context could not be created")]
    GraphicsCreationFailed(iced_graphics::Error),

    /// The application connection to the wayland server could not be created.
    #[error("The application connection to the wayland server could not be created.")]
    ConnectionCreationFailed(ConnectError),
}
