use std::time::Instant;

use crate::dpi::PhysicalSize;
use iced_graphics::Point;
use iced_native::{
    mouse::{self, ScrollDelta},
    window::Id as SurfaceId,
};
use sctk::{
    output::OutputInfo,
    reexports::client::backend::ObjectId,
    seat::{
        keyboard::{KeyEvent, Modifiers},
        pointer::{PointerEvent, PointerEventKind},
        Capability,
    },
    shell::{
        layer::LayerSurfaceConfigure,
        xdg::{popup::PopupConfigure, window::WindowConfigure},
    },
};

#[derive(Debug, Clone)]
pub enum IcedSctkEvent<T> {
    /// Emitted when new events arrive from the OS to be processed.
    ///
    /// This event type is useful as a place to put code that should be done before you start
    /// processing events, such as updating frame timing information for benchmarking or checking
    /// the [`StartCause`][crate::event::StartCause] to see if a timer set by
    /// [`ControlFlow::WaitUntil`](crate::event_loop::ControlFlow::WaitUntil) has elapsed.
    NewEvents(StartCause),

    /// Any user event from iced
    UserEvent(T),
    /// An event produced by sctk
    SctkEvent(SctkEvent),

    /// Emitted when all of the event loop's input events have been processed and redraw processing
    /// is about to begin.
    ///
    /// This event is useful as a place to put your code that should be run after all
    /// state-changing events have been handled and you want to do stuff (updating state, performing
    /// calculations, etc) that happens as the "main body" of your event loop. If your program only draws
    /// graphics when something changes, it's usually better to do it in response to
    /// [`Event::RedrawRequested`](crate::event::Event::RedrawRequested), which gets emitted
    /// immediately after this event. Programs that draw graphics continuously, like most games,
    /// can render here unconditionally for simplicity.
    MainEventsCleared,

    /// Emitted after [`MainEventsCleared`] when a window should be redrawn.
    ///
    /// This gets triggered in two scenarios:
    /// - The OS has performed an operation that's invalidated the window's contents (such as
    ///   resizing the window).
    /// - The application has explicitly requested a redraw via [`Window::request_redraw`].
    ///
    /// During each iteration of the event loop, Winit will aggregate duplicate redraw requests
    /// into a single event, to help avoid duplicating rendering work.
    ///
    /// Mainly of interest to applications with mostly-static graphics that avoid redrawing unless
    /// something changes, like most non-game GUIs.
    ///
    /// [`MainEventsCleared`]: Self::MainEventsCleared
    RedrawRequested(ObjectId),

    /// Emitted after all [`RedrawRequested`] events have been processed and control flow is about to
    /// be taken away from the program. If there are no `RedrawRequested` events, it is emitted
    /// immediately after `MainEventsCleared`.
    ///
    /// This event is useful for doing any cleanup or bookkeeping work after all the rendering
    /// tasks have been completed.
    ///
    /// [`RedrawRequested`]: Self::RedrawRequested
    RedrawEventsCleared,

    /// Emitted when the event loop is being shut down.
    ///
    /// This is irreversible - if this event is emitted, it is guaranteed to be the last event that
    /// gets emitted. You generally want to treat this as an "do on quit" event.
    LoopDestroyed,
}

#[derive(Debug, Clone)]
pub enum SctkEvent {
    //
    // Input events
    //
    SeatEvent {
        variant: SeatEventVariant,
        id: ObjectId,
    },
    PointerEvent {
        variant: PointerEvent,
        ptr_id: ObjectId,
        seat_id: ObjectId,
    },
    KeyboardEvent {
        variant: KeyboardEventVariant,
        kbd_id: ObjectId,
        seat_id: ObjectId,
    },
    // TODO data device & touch

    //
    // Surface Events
    //
    WindowEvent {
        variant: WindowEventVariant,
        id: ObjectId,
    },
    LayerSurfaceEvent {
        variant: LayerSurfaceEventVariant,
        id: ObjectId,
    },
    PopupEvent {
        variant: PopupEventVariant,
        /// this may be the Id of a window or layer surface
        toplevel_id: ObjectId,
        /// this may be any SurfaceId
        parent_id: ObjectId,
        /// the id of this popup
        id: ObjectId,
    },

    //
    // output events
    //
    NewOutput {
        id: ObjectId,
        info: Option<OutputInfo>,
    },
    UpdateOutput {
        id: ObjectId,
        info: OutputInfo,
    },
    RemovedOutput(ObjectId),

    //
    // compositor events
    //
    Draw(ObjectId),
    ScaleFactorChanged {
        factor: f64,
        id: ObjectId,
        inner_size: PhysicalSize<u32>,
    },
}

#[derive(Debug, Clone)]
pub enum SeatEventVariant {
    New,
    Remove,
    NewCapability(Capability, ObjectId),
    RemoveCapability(Capability, ObjectId),
}

#[derive(Debug, Clone)]
pub enum KeyboardEventVariant {
    Leave(ObjectId),
    Enter(ObjectId),
    Press(KeyEvent),
    Release(KeyEvent),
    Modifiers(Modifiers),
}

#[derive(Debug, Clone)]
pub enum WindowEventVariant {
    Created((ObjectId, SurfaceId)),
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:close>
    Close,
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:wm_capabilities>
    WmCapabilities(Vec<u32>),
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:configure_bounds>
    ConfigureBounds {
        width: u32,
        height: u32,
    },
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:configure>
    Configure(WindowConfigure),
}

#[derive(Debug, Clone)]
pub enum PopupEventVariant {
    Created((ObjectId, SurfaceId)),
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:popup_done>
    Done,
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:wm_capabilities>
    WmCapabilities(Vec<u32>),
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:configure>
    Configure(PopupConfigure),
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:repositioned>
    RepositionionedPopup {
        token: u32,
    },
}

#[derive(Debug, Clone)]
pub enum LayerSurfaceEventVariant {
    /// sent after creation of the layer surface
    Created((ObjectId, SurfaceId)),
    /// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:event:closed>
    Done,
    /// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:event:configure>
    Configure(LayerSurfaceConfigure),
}

/// Describes the reason the event loop is resuming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartCause {
    /// Sent if the time specified by [`ControlFlow::WaitUntil`] has been reached. Contains the
    /// moment the timeout was requested and the requested resume time. The actual resume time is
    /// guaranteed to be equal to or after the requested resume time.
    ///
    /// [`ControlFlow::WaitUntil`]: crate::event_loop::ControlFlow::WaitUntil
    ResumeTimeReached {
        start: Instant,
        requested_resume: Instant,
    },

    /// Sent if the OS has new events to send to the window, after a wait was requested. Contains
    /// the moment the wait was requested and the resume time, if requested.
    WaitCancelled {
        start: Instant,
        requested_resume: Option<Instant>,
    },

    /// Sent if the event loop is being resumed after the loop's control flow was set to
    /// [`ControlFlow::Poll`].
    ///
    /// [`ControlFlow::Poll`]: crate::event_loop::ControlFlow::Poll
    Poll,

    /// Sent once, immediately after `run` is called. Indicates that the loop was just initialized.
    Init,
}

/// Pending update to a window requested by the user.
#[derive(Default, Debug, Clone, Copy)]
pub struct SurfaceUserRequest {
    /// Whether `redraw` was requested.
    pub redraw_requested: bool,

    /// Wether the frame should be refreshed.
    pub refresh_frame: bool,
}

// The window update comming from the compositor.
#[derive(Default, Debug, Clone)]
pub struct SurfaceCompositorUpdate {
    /// New window configure.
    pub configure: Option<WindowConfigure>,

    /// New scale factor.
    pub scale_factor: Option<i32>,

    /// Close the window.
    pub close_window: bool,
}

impl SctkEvent {
    pub fn to_native(self) -> Option<iced_native::Event> {
        match self {
            SctkEvent::SeatEvent { variant, id } => None,
            SctkEvent::PointerEvent {
                variant,
                ptr_id,
                seat_id,
            } => match variant.kind {
                PointerEventKind::Enter { serial } => {
                    Some(iced_native::Event::Mouse(mouse::Event::CursorEntered))
                }
                PointerEventKind::Leave { serial } => {
                    Some(iced_native::Event::Mouse(mouse::Event::CursorLeft))
                }
                PointerEventKind::Motion { time } => {
                    Some(iced_native::Event::Mouse(mouse::Event::CursorMoved {
                        position: Point::new(variant.position.0 as f32, variant.position.1 as f32),
                    }))
                }
                PointerEventKind::Press {
                    time,
                    button,
                    serial,
                } => Some(iced_native::Event::Mouse(mouse::Event::ButtonPressed(
                    mouse::Button::Left,
                ))), // TODO Ashley: conversion
                PointerEventKind::Release {
                    time,
                    button,
                    serial,
                } => Some(iced_native::Event::Mouse(mouse::Event::ButtonReleased(
                    mouse::Button::Left,
                ))), // TODO Ashley: conversion
                PointerEventKind::Axis {
                    time,
                    horizontal,
                    vertical,
                    source,
                } => Some(iced_native::Event::Mouse(mouse::Event::WheelScrolled {
                    delta: ScrollDelta::Lines { x: 0.0, y: 1.0 },
                })), // TODO Ashley: conversion
            },
            SctkEvent::KeyboardEvent {
                variant,
                kbd_id,
                seat_id,
            } => match variant {
                KeyboardEventVariant::Leave(_) => todo!(),
                KeyboardEventVariant::Enter(_) => todo!(),
                KeyboardEventVariant::Press(_) => todo!(),
                KeyboardEventVariant::Release(_) => todo!(),
                KeyboardEventVariant::Modifiers(_) => todo!(),
            },
            SctkEvent::WindowEvent { variant, id } => None,
            SctkEvent::LayerSurfaceEvent { variant, id } => None,
            SctkEvent::PopupEvent {
                variant,
                toplevel_id,
                parent_id,
                id,
            } => None,
            SctkEvent::NewOutput { id, info } => None,
            SctkEvent::UpdateOutput { id, info } => None,
            SctkEvent::RemovedOutput(_) => None,
            SctkEvent::Draw(_) => None,
            SctkEvent::ScaleFactorChanged {
                factor,
                id,
                inner_size,
            } => None,
        }
    }
}
