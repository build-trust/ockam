use minicbor::Decoder;
use ockam_core::api::{RequestHeader, Response};
use ockam_core::{Result, Routed};
use ockam_node::Context;

/// Return a response on the return route stating that a secure channel is needed to access
/// the service
pub async fn secure_channel_required(c: &mut Context, m: Routed<Vec<u8>>) -> Result<()> {
    // This was, actually, already checked by the access control. So if we reach this point
    // it means there is a bug.  Also, if it' already checked, we should receive the Peer'
    // identity, not an Option to the peer' identity.
    let mut dec = Decoder::new(m.as_body());
    let req: RequestHeader = dec.decode()?;
    let res = Response::forbidden(&req, "secure channel required").to_vec()?;
    c.send(m.return_route(), res).await
}
