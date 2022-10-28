use crate::{
    event_loop::state::SctkState,
    sctk_event::{SctkEvent, WindowEventVariant},
};
use sctk::{delegate_xdg_window, reexports::client::Proxy, shell::xdg::window::WindowHandler};
use std::fmt::Debug;

impl<T: Debug> WindowHandler for SctkState<T> {
    fn request_close(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        window: &sctk::shell::xdg::window::Window,
    ) {
        let window = match self
            .windows
            .iter()
            .position(|s| s.window.wl_surface() == window.wl_surface())
        {
            Some(w) => self.windows.remove(w),
            None => return,
        };

        self.sctk_events.push(SctkEvent::WindowEvent {
            variant: WindowEventVariant::Close,
            id: window.window.wl_surface().id(),
        })
        // TODO popup cleanup
    }

    fn configure(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        window: &sctk::shell::xdg::window::Window,
        configure: sctk::shell::xdg::window::WindowConfigure,
        _serial: u32,
    ) {
        let window = match self
            .windows
            .iter_mut()
            .find(|w| w.window.wl_surface() == window.wl_surface())
        {
            Some(w) => w,
            None => return,
        };
        let first = window.last_configure.is_none();
        window.last_configure.replace(configure.clone());
        self.sctk_events.push(SctkEvent::WindowEvent {
            variant: WindowEventVariant::Configure(configure, first),
            id: window.window.wl_surface().id(),
        })
    }
}

delegate_xdg_window!(@<T: 'static + Debug> SctkState<T>);
