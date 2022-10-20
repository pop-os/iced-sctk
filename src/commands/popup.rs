//! Interact with the popups of your application.
use iced_native::{command::Command, window};

pub use window::{Event, Mode};

/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:get_popup>
/// <https://wayland.app/protocols/xdg-shell#xdg_surface:request:get_popup>
pub fn get_popup<Message>(width: u32, height: u32) -> Command<Message> {
    todo!();
}

/// <https://wayland.app/protocols/xdg-shell#xdg_popup:request:reposition>
pub fn reposition_popup<Message>(width: u32, height: u32) -> Command<Message> {
    todo!();
}

/// <https://wayland.app/protocols/xdg-shell#xdg_popup:request:reposition>
pub fn grab_popup<Message>(width: u32, height: u32) -> Command<Message> {
    todo!();
}

/// <https://wayland.app/protocols/xdg-shell#xdg_popup:request:destroy>
pub fn destroy_popup<Message>(mode: Mode) -> Command<Message> {
    todo!()
}
