use crate::{
    event_loop::state::{self, SctkState, SctkSurface},
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
        configure: sctk::shell::xdg::popup::PopupConfigure,
    ) {
        let sctk_popup = match self
            .popups
            .iter_mut()
            .find(|s| s.popup.wl_surface().id() == popup.wl_surface().id())
        {
            Some(p) => p,
            None => return,
        };
        let first = sctk_popup.last_configure.is_none();
        sctk_popup.last_configure.replace(configure.clone());

        self.sctk_events.push(SctkEvent::PopupEvent {
            variant: PopupEventVariant::Configure(configure, popup.wl_surface().clone(), first),
            id: popup.wl_surface().id(),
            toplevel_id: sctk_popup.toplevel.id(),
            parent_id: match &sctk_popup.parent {
                SctkSurface::LayerSurface(s) => s.id(),
                SctkSurface::Window(s) => s.id(),
                SctkSurface::Popup(s) => s.id(),
            },
        })
    }

    fn done(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        popup: &sctk::shell::xdg::popup::Popup,
    ) {
        let mut to_destroy = vec![popup.wl_surface().id()];
        while let Some(id) = to_destroy.last().cloned() {
            if let Some(i) = self
                .popups
                .iter()
                .position(|p| p.popup.wl_surface().id() == id)
            {
                let popup_to_destroy = self.popups.remove(i);
                match &popup_to_destroy.parent.clone() {
                    state::SctkSurface::LayerSurface(_) | state::SctkSurface::Window(_) => {
                        popup_to_destroy.popup.xdg_popup().destroy();
                        self.sctk_events.push(SctkEvent::PopupEvent {
                            variant: PopupEventVariant::Done,
                            toplevel_id: popup_to_destroy.toplevel.id(),
                            parent_id: popup_to_destroy.parent.wl_surface().id(),
                            id: popup_to_destroy.popup.wl_surface().id(),
                        });
                        to_destroy.pop();
                    }
                    state::SctkSurface::Popup(popup_to_destroy_first) => {
                        self.popups.push(popup_to_destroy);
                        let popup_to_destroy_first = self
                            .popups
                            .iter()
                            .find(|p| p.popup.wl_surface() == popup_to_destroy_first)
                            .unwrap();
                        to_destroy.push(popup_to_destroy_first.popup.wl_surface().id());
                    }
                }
            } else {
                to_destroy.pop();
            }
        }
    }
}
delegate_xdg_popup!(@<T: 'static + Debug> SctkState<T>);
