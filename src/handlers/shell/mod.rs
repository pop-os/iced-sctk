use sctk::delegate_xdg_shell;
use std::fmt::Debug;

use crate::event_loop::state::SctkState;

pub mod layer;
pub mod xdg_popup;
pub mod xdg_window;

delegate_xdg_shell!(@<T: 'static + Debug> SctkState<T>);
