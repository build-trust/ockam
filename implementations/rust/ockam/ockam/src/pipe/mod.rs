//! Ockam pipe module

mod internal;

mod sender;
pub use sender::PipeSender;
mod receiver;
pub use receiver::PipeReceiver;

use ockam_core::{Address, Result, Route};
use ockam_node::Context;

/// Connect to the receiving end of a pipe
///
/// Returns the PipeSender's public address.
pub async fn connect_static(ctx: &mut Context, recv: Route) -> Result<Address> {
    let addr = Address::random(0);
    PipeSender::create(ctx, recv, addr.clone(), Address::random(0))
        .await
        .map(|_| addr)
}

/// Connect to the pipe receive listener and then to a pipe receiver
pub async fn connect_dynamic(_listener: Route) -> PipeSender {
    todo!()
}

/// Create a receiver with a static address
pub async fn receiver(_addr: Address) -> PipeReceiver {
    todo!()
}

/// Create a pipe receive listener
///
/// This special worker will create pipe receivers for any incoming
/// connection.  The worker can simply be stopped via its address.
pub async fn listen_for_connections(_addr: Address) {}
