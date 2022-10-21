use std::{
    collections::HashMap,
    fmt::Debug,
    num::NonZeroU32,
    sync::{Arc, Mutex},
};

use crate::{
    dpi::LogicalSize,
    egl::init_egl,
    handlers::shell::xdg_window::WindowRequest,
    sctk_event::{IcedSctkEvent, SctkEvent, SurfaceCompositorUpdate, SurfaceUserRequest},
};
use futures::channel::mpsc;
use glutin::{
    api::egl::{self, config::Config, display::Display},
    display,
    prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor},
    surface::WindowSurface,
};
use iced_native::{keyboard::Modifiers, command::platform_specific::wayland::layer_surface::IcedMargin};
use sctk::{
    compositor::CompositorState,
    event_loop::WaylandSource,
    output::OutputState,
    reexports::{
        calloop::{self, LoopHandle},
        client::{
            backend::ObjectId,
            protocol::{
                wl_data_device::WlDataDevice,
                wl_display::WlDisplay,
                wl_keyboard::WlKeyboard,
                wl_output::WlOutput,
                wl_pointer::WlPointer,
                wl_seat::WlSeat,
                wl_surface::{self, WlSurface},
                wl_touch::WlTouch,
            },
            ConnectError, Connection, Proxy, QueueHandle,
        },
    },
    registry::RegistryState,
    seat::{keyboard::KeyEvent, SeatState},
    shell::{
        layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface, LayerSurfaceConfigure,
        },
        xdg::{
            popup::{Popup, PopupConfigure},
            window::{Window, WindowConfigure, XdgWindowState},
            XdgPositioner, XdgShellState, XdgShellSurface,
        },
    },
    shm::{multi::MultiPool, ShmState},
};

#[derive(Debug, Clone)]
pub(crate) struct SctkSeat {
    pub(crate) seat: WlSeat,
    pub(crate) kbd: Option<WlKeyboard>,
    pub(crate) kbd_focus: Option<WlSurface>,
    pub(crate) last_kbd_press: Option<KeyEvent>,
    pub(crate) ptr: Option<WlPointer>,
    pub(crate) ptr_focus: Option<WlSurface>,
    pub(crate) last_ptr_press: Option<(u32, u32, u32)>, // (time, button, serial)
    pub(crate) touch: Option<WlTouch>,
    pub(crate) data_device: Option<WlDataDevice>,
    pub(crate) modifiers: Modifiers,
}

#[derive(Debug, Clone)]
pub struct SctkWindow {
    pub(crate) window: Window,
    pub(crate) requested_size: Option<LogicalSize<u32>>,
    pub(crate) current_size: Option<LogicalSize<u32>>,
    pub(crate) last_configure: Option<WindowConfigure>,
    /// Requests that SCTK window should perform.
    pub(crate) pending_requests: Arc<Mutex<Vec<WindowRequest>>>,
    xdg_surface: Arc<XdgShellSurface>,
}

#[derive(Debug, Clone)]
pub struct SctkLayerSurface {
    pub(crate) surface: LayerSurface,
    pub(crate) requested_size: Option<LogicalSize<u32>>,
    pub(crate) current_size: Option<LogicalSize<u32>>,
    pub(crate) layer: Layer,
    pub(crate) anchor: Anchor,
    pub(crate) keyboard_interactivity: KeyboardInteractivity,
    pub(crate) margin: IcedMargin,
    pub(crate) exclusive_zone: i32,
    pub(crate) last_configure: Option<LayerSurfaceConfigure>,
}

#[derive(Debug, Clone)]
pub enum SctkSurface {
    LayerSurface(LayerSurface),
    Window(Window),
    Popup(Popup),
}

#[derive(Debug, Clone)]
pub struct SctkPopup {
    pub(crate) popup: Popup,
    pub(crate) parent: SctkSurface,
    pub(crate) toplevel: WlSurface,
    pub(crate) requested_size: Option<LogicalSize<u32>>,
    pub(crate) current_size: Option<LogicalSize<u32>>,
    pub(crate) last_configure: Option<PopupConfigure>,
    // pub(crate) positioner: XdgPositioner,
    xdg_surface: Arc<XdgShellSurface>,
}

/// Wrapper to carry sctk state.
#[derive(Debug)]
pub struct SctkState<T> {
    // egl
    // pub(crate) context: Option<egl::context::PossiblyCurrentContext>,
    // pub(crate) glow: Option<glow::Context>,
    // pub(crate) display: Option<Display>,
    // pub(crate) config: Option<glutin::api::egl::config::Config>,
    /// the cursor wl_surface
    pub(crate) cursor_surface: Option<wl_surface::WlSurface>,
    /// a memory pool
    pub(crate) multipool: Option<MultiPool<WlSurface>>,

    // all present outputs
    pub(crate) outputs: Vec<WlOutput>,
    // though (for now) only one seat will be active in an iced application at a time, all ought to be tracked
    pub(crate) seats: Vec<SctkSeat>,
    // Windows / Surfaces
    /// Window list containing all SCTK windows. Since those windows aren't allowed
    /// to be sent to other threads, they live on the event loop's thread
    /// and requests from winit's windows are being forwarded to them either via
    /// `WindowUpdate` or buffer on the associated with it `WindowHandle`.
    pub(crate) windows: HashMap<
        ObjectId,
            SctkWindow,
    >,
    pub(crate) layer_surfaces: HashMap<
        ObjectId,
            SctkLayerSurface,
    >,
    pub(crate) popups: HashMap<
        ObjectId,
            SctkPopup,
    >,
    pub(crate) kbd_focus: Option<WlSurface>,

    /// Window updates, which are coming from SCTK or the compositor, which require
    /// calling back to the sctk's downstream. They are handled right in the event loop,
    /// unlike the ones coming from buffers on the `WindowHandle`'s.
    pub popup_compositor_updates: HashMap<ObjectId, SurfaceCompositorUpdate>,
    /// Window updates, which are coming from SCTK or the compositor, which require
    /// calling back to the sctk's downstream. They are handled right in the event loop,
    /// unlike the ones coming from buffers on the `WindowHandle`'s.
    pub window_compositor_updates: HashMap<ObjectId, SurfaceCompositorUpdate>,
    /// Layer Surface updates, which are coming from SCTK or the compositor, which require
    /// calling back to the sctk's downstream. They are handled right in the event loop,
    /// unlike the ones coming from buffers on the `WindowHandle`'s.
    pub layer_surface_compositor_updates: HashMap<ObjectId, SurfaceCompositorUpdate>,

    /// A sink for window and device events that is being filled during dispatching
    /// event loop and forwarded downstream afterwards.
    pub(crate) sctk_events: Vec<SctkEvent>,
    /// Window updates comming from the user requests. Those are separatelly dispatched right after
    /// `MainEventsCleared`.
    pub window_user_requests: HashMap<ObjectId, SurfaceUserRequest>,
    /// Layer Surface updates comming from the user requests. Those are separatelly dispatched right after
    /// `MainEventsCleared`.
    pub layer_surface_user_requests: HashMap<ObjectId, SurfaceUserRequest>,
    /// Window updates comming from the user requests. Those are separatelly dispatched right after
    /// `MainEventsCleared`.
    pub popup_user_requests: HashMap<ObjectId, SurfaceUserRequest>,

    /// pending user events
    pub pending_user_events: Vec<(iced_native::window::Id, T)>,

    // handles
    pub(crate) queue_handle: QueueHandle<Self>,
    pub(crate) loop_handle: LoopHandle<'static, Self>,

    // sctk state objects
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) compositor_state: CompositorState,
    pub(crate) shm_state: ShmState,
    pub(crate) xdg_shell_state: XdgShellState,
    pub(crate) xdg_window_state: XdgWindowState,
    pub(crate) layer_shell: LayerShell,

    pub(crate) connection: Connection,
}

// impl<T> SctkState<T>
// where
//     T: 'static + Debug,
// {
//     pub fn get_surface(
//         &mut self,
//         surface: &wl_surface::WlSurface,
//         width: u32,
//         height: u32,
//     ) -> egl::surface::Surface<glutin::surface::WindowSurface> {
//         if let (Some(display), Some(config)) = (self.display.as_ref(), self.config.as_ref()) {
//             let mut window_handle = raw_window_handle::WaylandWindowHandle::empty();
//             window_handle.surface = surface.id().as_ptr() as *mut _;
//             let window_handle = raw_window_handle::RawWindowHandle::Wayland(window_handle);
//             let surface_attrs =
//                 glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::default().build(
//                     window_handle,
//                     NonZeroU32::new(width).unwrap(),
//                     NonZeroU32::new(height).unwrap(),
//                 );
//             let surface = unsafe { display.create_window_surface(&config, &surface_attrs) }
//                 .expect("Failed to create surface");
//             surface
//         } else {
//             let (display, context, config, surface) = init_egl(surface, width, height);
//             let context = context.make_current(&surface).unwrap();

//             self.display.replace(display);
//             self.context.replace(context);
//             self.config.replace(config);
//             surface
//         }
//     }
// }
