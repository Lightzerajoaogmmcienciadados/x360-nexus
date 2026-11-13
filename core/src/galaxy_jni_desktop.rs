//! Desktop-safe neutral controller state for the currently partial HLE.
 //! Android touch/USB input uses galaxy_jni.rs. This module intentionally
 //! does not invent Windows XInput state.
use crate::input::Controller;
pub(crate) fn controller_snapshot() -> Controller { Controller::default() }
