//! Interact with the windows of your application.
use iced_native::{
    command::{self, platform_specific, Command},
    window,
};

use sctk::shell::xdg::window::{Window, WindowBuilder};
use wayland_backend::client::ObjectId;
pub use window::{Event, Mode};

/// Get a new window for your application
/// <https://wayland.app/protocols/xdg-shell#xdg_surface:request:get_toplevel>
pub fn get_window<Message>(
    builder: WindowBuilder,
    o: impl FnOnce(ObjectId) -> Message + 'static,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(platform_specific::wayland::Action::Window(
            platform_specific::wayland::window::Action::Window {
                builder,
                o: Box::new(o),
            },
        )),
    ))
}

/// Resizes the window to the given logical dimensions.
pub fn resize<Message>(id: ObjectId, width: u32, height: u32) -> Command<Message> {
    Command::single(command::Action::Window(window::Action::Resize {
        width,
        height,
    }))
}

/// Sets the [`Mode`] of the window.
/// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_minimized>
/// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_fullscreen>
/// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:unset_fullscreen>
pub fn set_mode<Message>(id: ObjectId, mode: Mode) -> Command<Message> {
    Command::single(command::Action::Window(window::Action::SetMode(mode)))
}

/// Fetches the current [`Mode`] of the window.
pub fn fetch_mode<Message>(
    id: ObjectId,
    f: impl FnOnce(Mode) -> Message + 'static,
) -> Command<Message> {
    Command::single(command::Action::Window(window::Action::FetchMode(
        Box::new(f),
    )))
}

/// drags the window
/// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:move>
pub fn drag<Message>(width: u32, height: u32) -> Command<Message> {
    todo!();
}

/// resize the window with a drag
/// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:resize>
pub fn resize_drag<Message>(width: u32, height: u32) -> Command<Message> {
    todo!();
}

/// maximize the window
/// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_maximized>
/// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_maximized>
pub fn set_maximized<Message>(width: u32, height: u32) -> Command<Message> {
    todo!();
}

/// fullscreen the window
pub fn fullscreen<Message>(width: u32, height: u32) -> Command<Message> {
    todo!();
}

/// Destroys the window.
/// https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:destroy
pub fn destroy_window<Message>(mode: Mode) -> Command<Message> {
    todo!()
}
