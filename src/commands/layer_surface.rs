//! Interact with the window of your application.
use std::marker::PhantomData;

use iced_native::window::Id as SurfaceId;
use iced_native::{
    command::{
        self,
        platform_specific::{
            self,
            wayland::{self, layer_surface::IcedLayerSurface},
        },
        Command,
    },
    window,
};
pub use window::{Event, Mode};

pub use sctk::shell::layer::{Anchor, KeyboardInteractivity, Layer};

// TODO implement as builder that outputs a batched commands
/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_shell_v1:request:get_layer_surface>
pub fn get_layer_surface<Message>(builder: IcedLayerSurface) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::LayerSurface(
            wayland::layer_surface::Action::LayerSurface {
                builder,
                _phantom: PhantomData::default(),
            },
        )),
    ))
}

/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:destroy>
pub fn destroy_layer_surface<Message>(id: SurfaceId) -> Command<Message> {
    todo!()
}

/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:set_size>
pub fn set_size<Message>(id: SurfaceId, width: u32, height: u32) -> Command<Message> {
    todo!()
}
/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:set_anchor>
pub fn set_anchor<Message>(id: SurfaceId, anchor: Anchor) -> Command<Message> {
    todo!()
}
/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:set_exclusive_zone>
pub fn set_exclusive_zone<Message>(id: SurfaceId, zone: i32) -> Command<Message> {
    todo!()
}

/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:set_margin>
pub fn set_margin<Message>(
    id: SurfaceId,
    top: u32,
    right: u32,
    bottom: u32,
    left: u32,
) -> Command<Message> {
    todo!()
}

/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:set_keyboard_interactivity>
pub fn set_keyboard_interactivity<Message>(
    id: SurfaceId,
    keyboard_interactivity: KeyboardInteractivity,
) -> Command<Message> {
    todo!()
}

/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:set_layer>
pub fn set_layer<Message>(id: SurfaceId, layer: Layer) -> Command<Message> {
    todo!()
}
