use minicbor::Decoder;

use ockam::remote::RemoteForwarder;
use ockam::Result;
use ockam_core::api::{Request, Response, Status};
use ockam_node::Context;

use crate::error::ApiError;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::nodes::NodeManager;

impl NodeManager {
    pub(super) async fn create_forwarder(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let CreateForwarder {
            address,
            alias,
            at_rust_node,
            ..
        } = dec.decode()?;
        let route = crate::multiaddr_to_route(&address)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;
        debug!(%address, ?alias, "Handling CreateForwarder request");

        let forwarder = match alias {
            Some(alias) => {
                if at_rust_node {
                    RemoteForwarder::create_static_without_heartbeats(ctx, route, alias.to_string())
                        .await
                } else {
                    RemoteForwarder::create_static(ctx, route, alias.to_string()).await
                }
            }
            None => RemoteForwarder::create(ctx, route).await,
        };

        match forwarder {
            Ok(info) => {
                let b = ForwarderInfo::from(info);
                debug!(
                    forwarding_route = %b.forwarding_route(),
                    remote_address = %b.remote_address(),
                    "CreateForwarder request processed, sending back response"
                );
                Ok(Response::ok(req.id()).body(b).to_vec()?)
            }
            Err(err) => {
                error!(?err, "Failed to create forwarder");
                Ok(Response::builder(req.id(), Status::InternalServerError)
                    .body(err.to_string())
                    .to_vec()?)
            }
        }
    }
}
