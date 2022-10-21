//! Interact with the window of your application.
use crate::command::{self, Command};
use iced_native::window;

pub use window::{Action, Id};
pub use window::{Event, Mode};

/// TODO(derezzedex)
pub fn close<Message>(id: window::Id) -> Command<Message> {
    Command::single(command::Action::Window(id, Action::Close))
}

/// Resizes the window to the given logical dimensions.
pub fn resize<Message>(id: window::Id, width: u32, height: u32) -> Command<Message> {
    Command::single(command::Action::Window(
        id,
        Action::Resize { width, height },
    ))
}

/// Moves a window to the given logical coordinates.
pub fn move_to<Message>(id: window::Id, x: i32, y: i32) -> Command<Message> {
    Command::single(command::Action::Window(id, Action::Move { x, y }))
}

/// Sets the [`Mode`] of the window.
pub fn set_mode<Message>(id: window::Id, mode: Mode) -> Command<Message> {
    Command::single(command::Action::Window(id, Action::SetMode(mode)))
}

/// Fetches the current [`Mode`] of the window.
pub fn fetch_mode<Message>(
    id: window::Id,
    f: impl FnOnce(Mode) -> Message + 'static,
) -> Command<Message> {
    Command::single(command::Action::Window(id, Action::FetchMode(Box::new(f))))
}
