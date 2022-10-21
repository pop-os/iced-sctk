// handlers
pub mod compositor;
pub mod data_device;
pub mod output;
pub mod seat;
pub mod shell;

use sctk::{
    delegate_registry, delegate_shm,
    output::OutputState,
    reexports::client::{
        globals::GlobalListContents, protocol::wl_registry, Connection, Dispatch, QueueHandle,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::SeatState,
    shm::{ShmHandler, ShmState},
};
use std::fmt::Debug;

use crate::event_loop::state::SctkState;

// Most of these handlers have not been properly filled out.
//
// The idea is for each of these to track what needs to be tracked in the SctkState,
// then send a message for each event to the Sender for the Iced Application to handle
//

impl<T: Debug> ShmHandler for SctkState<T> {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}

impl<T: Debug> ProvidesRegistryState for SctkState<T>
where
    T: 'static,
{
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

delegate_shm!(@<T: 'static + Debug> SctkState<T>);
delegate_registry!(@<T: 'static + Debug> SctkState<T>);

impl<T: Debug> Dispatch<wl_registry::WlRegistry, GlobalListContents> for SctkState<T> {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We don't need any other globals.
    }
}
