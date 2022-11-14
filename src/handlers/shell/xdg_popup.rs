use crate::{
    event_loop::state::{self, SctkState, SctkSurface},
    sctk_event::{PopupEventVariant, SctkEvent}, commands::popup,
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
        let sctk_popup = match self
            .popups
            .iter()
            .position(|s| s.popup.wl_surface().id() == popup.wl_surface().id())
        {
            Some(p) => self.popups.remove(p),
            None => return,
        };
        let mut to_destroy = vec![sctk_popup];
        while let Some(popup_to_destroy) = to_destroy.last() {
            match popup_to_destroy.parent.clone() {
                state::SctkSurface::LayerSurface(_) | state::SctkSurface::Window(_) => {
                    break;
                }
                state::SctkSurface::Popup(popup_to_destroy_first) => {
                    let popup_to_destroy_first = self
                        .popups
                        .iter()
                        .position(|p| p.popup.wl_surface() == &popup_to_destroy_first)
                        .unwrap();
                    let popup_to_destroy_first = self.popups.remove(popup_to_destroy_first);
                    to_destroy.push(popup_to_destroy_first);
                }
            }
        }
        for popup in to_destroy.into_iter().rev() {
            self.sctk_events.push(SctkEvent::PopupEvent {
                variant: PopupEventVariant::Done,
                toplevel_id: popup.toplevel.id(),
                parent_id: popup.parent.wl_surface().id(),
                id: popup.popup.wl_surface().id(),
            });
            self.popups.push(popup);
        }
    }
}
delegate_xdg_popup!(@<T: 'static + Debug> SctkState<T>);
