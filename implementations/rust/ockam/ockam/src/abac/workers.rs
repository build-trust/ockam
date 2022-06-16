use crate::{
    abac::{AbacLocalInfo, AbacMetadata, ABAC_LOCAL_INFO_IDENTIFIER},
    Any, Context, OckamMessage, Result, Route, Routed, Worker,
};
use ockam_core::compat::boxed::Box;
use ockam_core::{Decodable, LocalMessage};

/// An `AbacUnwrapperWorker` wraps any [`AbacLocalInfo`] attached to a
/// [`LocalMessage`] in `OckamMessage` [`Metadata`] so that it can be
/// recovered after being sent to another Ockam node.
pub struct AbacWrapperWorker {
    route: Route,
}

impl AbacWrapperWorker {
    /// Create a new `AbacWrapperWorker` with the given forwarding [`Route`]
    pub fn new(route: impl Into<Route>) -> Self {
        Self {
            route: route.into(),
        }
    }
}

#[crate::worker]
impl Worker for AbacWrapperWorker {
    type Context = Context;
    type Message = Any;

    /// Self::Message => OckamMessage
    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // Convert AbacLocalInfo to AbacMetadata
        let mut local_msg = msg.into_local_message();
        let abac_local_info = AbacLocalInfo::find_info(&local_msg)?;
        let abac_metadata = AbacMetadata::from(abac_local_info);

        // Clear AbacLocalInfo entries
        local_msg.clear_local_info(ABAC_LOCAL_INFO_IDENTIFIER);

        // Modify onward route
        let transport_msg = local_msg.transport_mut();
        transport_msg.onward_route.step()?;
        transport_msg
            .onward_route
            .modify()
            .prepend_route(self.route.clone());
        let onward_route = transport_msg.onward_route.clone();

        // Wrap in OckamMessage and attach Metadata
        let ockam_msg = abac_metadata.into_ockam_message(local_msg)?;

        // Send to next hop
        ctx.send(onward_route, ockam_msg).await
    }
}

/// An `AbacUnwrapperWorker` recovers any `AbacLocalInfo` embedded in
/// [`Metadata`] received from another Ockam node.
pub struct AbacUnwrapperWorker;

#[crate::worker]
impl Worker for AbacUnwrapperWorker {
    type Context = Context;
    type Message = OckamMessage;

    /// OckamMessage => LocalMessage
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        // recover wrapped message
        let ockam_msg: OckamMessage = msg.clone();
        let mut orig_local_msg = LocalMessage::decode(&ockam_msg.data)?;

        // Convert AbacMetadata to AbacLocalInfo
        let abac_metadata = AbacMetadata::find_metadata(&ockam_msg)?;
        let abac_local_info = AbacLocalInfo::from(abac_metadata);
        let local_info = abac_local_info.try_into()?;

        // Attach LocalInfo
        orig_local_msg.replace_local_info(local_info);

        // modify onward route
        let mut local_msg = msg.into_local_message();
        let transport_msg = local_msg.transport_mut();
        transport_msg.onward_route.step()?;
        let orig_transport_msg = orig_local_msg.transport_mut();
        orig_transport_msg.onward_route = transport_msg.onward_route.clone();

        // modify return route
        let return_route: Route = transport_msg.return_route.modify().pop_back().into();
        orig_transport_msg
            .return_route
            .modify()
            .prepend_route(return_route);

        // Forward to destination
        ctx.forward(orig_local_msg).await
    }
}
