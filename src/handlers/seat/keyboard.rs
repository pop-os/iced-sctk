use crate::{
    event_loop::state::SctkState,
    sctk_event::{KeyboardEventVariant, SctkEvent},
};

use sctk::{delegate_keyboard, reexports::client::Proxy, seat::keyboard::KeyboardHandler};
use std::fmt::Debug;

impl<T: Debug> KeyboardHandler for SctkState<T> {
    fn enter(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        surface: &sctk::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[u32],
    ) {
        let (is_active, my_seat) = match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
            if s.kbd.as_ref() == Some(keyboard) {
                Some((i, s))
            } else {
                None
            }
        }) {
            Some((i, s)) => (i == 0, s),
            None => return,
        };

        my_seat.kbd_focus.replace(surface.clone());

        if is_active {
            self.sctk_events.push(SctkEvent::KeyboardEvent {
                variant: KeyboardEventVariant::Enter(surface.id()),
                kbd_id: keyboard.id(),
                seat_id: my_seat.seat.id(),
            })
        }
    }

    fn leave(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        surface: &sctk::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
    ) {
        let (is_active, my_seat) = match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
            if s.kbd.as_ref() == Some(keyboard) {
                Some((i, s))
            } else {
                None
            }
        }) {
            Some((i, s)) => (i == 0, s),
            None => return,
        };
        let seat_id = my_seat.seat.id();
        let kbd_id = keyboard.id();
        let surface_id = surface.id();
        my_seat.kbd_focus.replace(surface.clone());

        if is_active {
            self.sctk_events.push(SctkEvent::KeyboardEvent {
                variant: KeyboardEventVariant::Leave(surface.id()),
                kbd_id,
                seat_id,
            })
        }
    }

    fn press_key(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: sctk::seat::keyboard::KeyEvent,
    ) {
        let (is_active, my_seat) = match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
            if s.kbd.as_ref() == Some(keyboard) {
                Some((i, s))
            } else {
                None
            }
        }) {
            Some((i, s)) => (i == 0, s),
            None => return,
        };
        let seat_id = my_seat.seat.id();
        let kbd_id = keyboard.id();
        my_seat.last_kbd_press.replace(event.clone());
        if is_active {
            self.sctk_events.push(SctkEvent::KeyboardEvent {
                variant: KeyboardEventVariant::Press(event),
                kbd_id,
                seat_id,
            });
        }
    }

    fn release_key(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: sctk::seat::keyboard::KeyEvent,
    ) {
        let (is_active, my_seat) = match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
            if s.kbd.as_ref() == Some(keyboard) {
                Some((i, s))
            } else {
                None
            }
        }) {
            Some((i, s)) => (i == 0, s),
            None => return,
        };
        let seat_id = my_seat.seat.id();
        let kbd_id = keyboard.id();

        if is_active {
            self.sctk_events.push(SctkEvent::KeyboardEvent {
                variant: KeyboardEventVariant::Release(event),
                kbd_id,
                seat_id,
            });
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: sctk::seat::keyboard::Modifiers,
    ) {
        let (is_active, my_seat) = match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
            if s.kbd.as_ref() == Some(keyboard) {
                Some((i, s))
            } else {
                None
            }
        }) {
            Some((i, s)) => (i == 0, s),
            None => return,
        };
        let seat_id = my_seat.seat.id();
        let kbd_id = keyboard.id();

        if is_active {
            self.sctk_events.push(SctkEvent::KeyboardEvent {
                variant: KeyboardEventVariant::Modifiers(modifiers),
                kbd_id,
                seat_id,
            })
        }
    }
}

delegate_keyboard!(@<T: 'static + Debug> SctkState<T>);
