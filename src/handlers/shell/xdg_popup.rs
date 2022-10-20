use crate::{
    event_loop::state::{SctkState, SctkSurface},
    sctk_event::{PopupEventVariant, SctkEvent},
};
use sctk::{delegate_xdg_popup, reexports::client::Proxy, shell::xdg::popup::PopupHandler};
use std::fmt::Debug;

impl<T: Debug> PopupHandler for SctkState<T> {
    fn configure(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        popup: &sctk::shell::xdg::popup::Popup,
        config: sctk::shell::xdg::popup::PopupConfigure,
    ) {
        let sctk_popup = match self.popups.get_mut(&popup.wl_surface().id()) {
            Some(p) => p,
            None => return,
        };
        self.sctk_events.push(SctkEvent::PopupEvent {
            variant: PopupEventVariant::Configure(config),
            id: popup.wl_surface().id(),
            toplevel_id: sctk_popup.0.toplevel.id(),
            parent_id: match &sctk_popup.0.parent {
                SctkSurface::LayerSurface(s) => s.wl_surface().id(),
                SctkSurface::Window(s) => s.wl_surface().id(),
                SctkSurface::Popup(s) => s.wl_surface().id(),
            },
        })
    }

    fn done(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        popup: &sctk::shell::xdg::popup::Popup,
    ) {
        let sctk_popup = match self.popups.remove(&popup.wl_surface().id()) {
            Some(p) => p,
            None => return,
        };
        self.sctk_events.push(SctkEvent::PopupEvent {
            variant: PopupEventVariant::Done,
            id: popup.wl_surface().id(),
            toplevel_id: sctk_popup.0.toplevel.id(),
            parent_id: match &sctk_popup.0.parent {
                SctkSurface::LayerSurface(s) => s.wl_surface().id(),
                SctkSurface::Window(s) => s.wl_surface().id(),
                SctkSurface::Popup(s) => s.wl_surface().id(),
            },
        })
        // TODO popup cleanup
    }
}
delegate_xdg_popup!(@<T: 'static + Debug> SctkState<T>);
