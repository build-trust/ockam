mod error;

// This directive makes sure that we only run the handshake protocol if it has been compiled
// on a little endian system since it is not supporting a big endian one at the moment
#[cfg(not(target_endian = "little"))]
compile_error!("Key Exchange is only supported on little-endian machines");

#[allow(clippy::module_inception)]
pub(crate) mod handshake;
pub(crate) mod handshake_state_machine;
pub(crate) mod handshake_worker;
mod initiator_state_machine;
mod responder_state_machine;
