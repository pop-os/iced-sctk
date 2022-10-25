use crate::{
    dpi::LogicalSize,
    event_loop::state::SctkState,
    sctk_event::{LayerSurfaceEventVariant, SctkEvent},
};
use sctk::{
    delegate_layer,
    reexports::client::Proxy,
    shell::layer::{Anchor, KeyboardInteractivity, LayerShellHandler},
};
use std::fmt::Debug;

impl<T: Debug> LayerShellHandler for SctkState<T> {
    fn closed(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        layer: &sctk::shell::layer::LayerSurface,
    ) {
        let layer = match self
            .layer_surfaces
            .iter()
            .position(|s| s.surface.wl_surface().id() == layer.wl_surface().id())
        {
            Some(w) => self.layer_surfaces.remove(w),
            None => return,
        };

        self.sctk_events.push(SctkEvent::LayerSurfaceEvent {
            variant: LayerSurfaceEventVariant::Done,
            id: layer.surface.wl_surface().id(),
        })
        // TODO popup cleanup
    }

    fn configure(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        layer: &sctk::shell::layer::LayerSurface,
        configure: sctk::shell::layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let layer = match self
            .layer_surfaces
            .iter_mut()
            .find(|s| s.surface.wl_surface().id() == layer.wl_surface().id())
        {
            Some(l) => l,
            None => return,
        };
        let id = layer.surface.wl_surface().id();
        self.sctk_events.push(SctkEvent::LayerSurfaceEvent {
            variant: LayerSurfaceEventVariant::Configure(configure),
            id: id.clone(),
        });
        self.sctk_events.push(SctkEvent::Draw(id));
    }
}

delegate_layer!(@<T: 'static + Debug> SctkState<T>);

/// A request to SCTK window from Winit window.
#[derive(Debug, Clone)]
pub enum LayerSurfaceRequest {
    /// Set fullscreen.
    ///
    /// Passing `None` will set it on the current monitor.
    Size(LogicalSize<u32>),

    /// Unset fullscreen.
    UnsetFullscreen,

    /// Show cursor for the certain window or not.
    ShowCursor(bool),

    /// Set anchor
    Anchor(Anchor),

    /// Set margin
    ExclusiveZone(i32),

    /// Set margin
    Margin(u32),

    /// Passthrough mouse input to underlying windows.
    KeyboardInteractivity(KeyboardInteractivity),

    /// Redraw was requested.
    Redraw,

    /// Window should be closed.
    Close,
}
