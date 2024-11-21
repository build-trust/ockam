#![allow(clippy::unconditional_recursion)]
use ockam_core::api::{Error, Response};
use ockam_core::flow_control::FlowControlId;
use ockam_core::Result;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::fmt::Display;

use super::NodeManagerWorker;
use crate::nodes::models::flow_controls::AddConsumer;
use crate::nodes::NodeManager;
use crate::LocalMultiaddrResolver;

impl NodeManagerWorker {
    pub(super) async fn add_consumer(
        &self,
        ctx: &Context,
        consumer: AddConsumer,
    ) -> Result<Response, Response<Error>> {
        match self
            .node_manager
            .add_consumer(ctx, consumer.address(), consumer.flow_control_id())
            .await
        {
            Ok(None) => Ok(Response::ok()),
            Ok(Some(failure)) => Err(Response::bad_request_no_request(&failure.to_string())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }
}

impl NodeManager {
    /// Add a consumer address for a given flow control id
    /// The given multiaddress must correspond to a route with only one Address
    /// otherwise a  AddConsumerError is returned
    pub async fn add_consumer(
        &self,
        ctx: &Context,
        consumer: &MultiAddr,
        flow_control_id: &FlowControlId,
    ) -> Result<Option<AddConsumerError>> {
        let mut route = LocalMultiaddrResolver::resolve(consumer)?;

        let address = match route.step().ok() {
            Some(a) => a,
            None => return Ok(Some(AddConsumerError::EmptyAddress(consumer.clone()))),
        };
        if !route.is_empty() {
            return Ok(Some(AddConsumerError::InvalidAddress(consumer.clone())));
        };

        ctx.flow_controls().add_consumer(address, flow_control_id);

        Ok(None)
    }
}

pub enum AddConsumerError {
    InvalidAddress(MultiAddr),
    EmptyAddress(MultiAddr),
}

impl Display for AddConsumerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddConsumerError::EmptyAddress(address) => {
                write!(
                    f,
                    "Unable to extract an address from the route: {address:?}."
                )
            }
            AddConsumerError::InvalidAddress(address) => write!(f, "Invalid address: {address:?}."),
        }
    }
}
