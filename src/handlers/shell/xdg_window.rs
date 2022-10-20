use crate::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::state::SctkState,
    sctk_event::{SctkEvent, WindowEventVariant},
    util::{CursorGrabMode, CursorIcon, UserAttentionType},
};
use sctk::{
    delegate_xdg_window,
    reexports::client::{protocol::wl_output::WlOutput, Proxy},
    shell::xdg::window::WindowHandler,
};
use std::fmt::Debug;
use wayland_backend::client::ObjectId;

impl<T: Debug> WindowHandler for SctkState<T> {
    fn request_close(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        window: &sctk::shell::xdg::window::Window,
    ) {
        let window = match self.windows.remove(&window.wl_surface().id()) {
            Some(w) => w,
            None => return,
        };

        self.sctk_events.push(SctkEvent::WindowEvent {
            variant: WindowEventVariant::Close,
            id: window.0.window.wl_surface().id(),
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
        let window = match self.windows.get_mut(&window.wl_surface().id()) {
            Some(w) => w,
            None => return,
        };
        self.sctk_events.push(SctkEvent::WindowEvent {
            variant: WindowEventVariant::Configure(configure),
            id: window.0.window.wl_surface().id(),
        })
    }
}

delegate_xdg_window!(@<T: 'static + Debug> SctkState<T>);

/// A request to SCTK window from Winit window.
#[derive(Debug, Clone)]
pub enum WindowRequest {
    /// Set fullscreen.
    ///
    /// Passing `None` will set it on the current monitor.
    Fullscreen(Option<WlOutput>),

    /// Unset fullscreen.
    UnsetFullscreen,

    /// Show cursor for the certain window or not.
    ShowCursor(bool),

    /// Change the cursor icon.
    NewCursorIcon(CursorIcon),

    /// Change cursor grabbing mode.
    SetCursorGrabMode(CursorGrabMode),

    /// Set cursor position.
    SetLockedCursorPosition(LogicalPosition<u32>),

    /// Drag window.
    DragWindow,

    /// Maximize the window.
    Maximize(bool),

    /// Minimize the window.
    Minimize,

    /// Request decorations change.
    Decorate(bool),

    /// Request decorations change.
    CsdThemeVariant(crate::util::Theme),

    /// Make the window resizeable.
    Resizeable(bool),

    /// Set the title for window.
    Title(String),

    /// Min size.
    MinSize(Option<LogicalSize<u32>>),

    /// Max size.
    MaxSize(Option<LogicalSize<u32>>),

    /// New frame size.
    FrameSize(LogicalSize<u32>),

    /// Set IME window position.
    ImePosition(LogicalPosition<u32>),

    /// Enable IME on the given window.
    AllowIme(bool),

    /// Request Attention.
    ///
    /// `None` unsets the attention request.
    Attention(Option<UserAttentionType>),

    /// Passthrough mouse input to underlying windows.
    PassthroughMouseInput(bool),

    /// Redraw was requested.
    Redraw,

    /// Window should be closed.
    Close,
}

pub fn handle_window_requests<T: 'static + Debug>(state: &mut SctkState<T>) {
    let window_map = &mut state.windows;
    let window_user_requests = &mut state.window_user_requests;
    let window_compositor_updates = &mut state.window_compositor_updates;
    let mut windows_to_close: Vec<ObjectId> = Vec::new();

    // Process the rest of the events.
    for (window_id, (window_handle, _)) in window_map.iter_mut() {
        let mut requests = window_handle.pending_requests.lock().unwrap();
        let requests = requests.drain(..);
        for request in requests {
            match request {
                WindowRequest::Fullscreen(fullscreen) => {
                    window_handle.window.set_fullscreen(fullscreen.as_ref());
                }
                WindowRequest::UnsetFullscreen => {
                    window_handle.window.unset_fullscreen();
                }
                WindowRequest::ShowCursor(show_cursor) => {
                    todo!()
                }
                WindowRequest::NewCursorIcon(cursor_icon) => {
                    todo!()
                }
                WindowRequest::ImePosition(position) => {
                    todo!()
                }
                WindowRequest::AllowIme(allow) => {
                    todo!()
                }
                WindowRequest::SetCursorGrabMode(mode) => {
                    todo!()
                }
                WindowRequest::SetLockedCursorPosition(position) => {
                    todo!()
                }
                WindowRequest::DragWindow => {
                    todo!()
                }
                WindowRequest::Maximize(maximize) => {
                    if maximize {
                        window_handle.window.set_maximized();
                    } else {
                        window_handle.window.unset_maximized();
                    }
                }
                WindowRequest::Minimize => {
                    todo!()
                }
                WindowRequest::Decorate(decorate) => {
                    todo!()
                }
                WindowRequest::CsdThemeVariant(_) => {}
                WindowRequest::Resizeable(resizeable) => {
                    todo!()
                }
                WindowRequest::Title(title) => {
                    window_handle.window.set_title(title);

                    // We should refresh the frame to draw new title.
                    let window_request = window_user_requests.get_mut(window_id).unwrap();
                    window_request.refresh_frame = true;
                }
                WindowRequest::MinSize(size) => {
                    let size = size.map(|size| (size.width, size.height));
                    window_handle.window.set_min_size(size);

                    let window_request = window_user_requests.get_mut(window_id).unwrap();
                    window_request.refresh_frame = true;
                }
                WindowRequest::MaxSize(size) => {
                    let size = size.map(|size| (size.width, size.height));
                    window_handle.window.set_max_size(size);

                    let window_request = window_user_requests.get_mut(window_id).unwrap();
                    window_request.refresh_frame = true;
                }
                WindowRequest::FrameSize(size) => {
                    todo!()
                }
                WindowRequest::PassthroughMouseInput(passthrough) => {
                    todo!()
                }
                WindowRequest::Attention(request_type) => {
                    todo!()
                }
                WindowRequest::Redraw => {
                    let window_request = window_user_requests.get_mut(window_id).unwrap();
                    window_request.redraw_requested = true;
                }
                WindowRequest::Close => {
                    todo!()
                }
            };
        }
    }

    // Close the windows.
    for window in windows_to_close {
        let _ = window_map.remove(&window);
        let _ = window_user_requests.remove(&window);
        let _ = window_compositor_updates.remove(&window);
    }
}
