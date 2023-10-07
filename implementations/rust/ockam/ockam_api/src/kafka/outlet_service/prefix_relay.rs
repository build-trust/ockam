use crate::kafka::KAFKA_OUTLET_CONSUMERS;

use crate::DefaultAddress;
use core::str::from_utf8;
use ockam::{Context, Result, Routed, Worker};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, AllowAll, AllowOnwardAddress};

/// This service applies a prefix to the provided static forwarding address.
/// This service was created mainly to keep full compatibility with the existing
/// erlang implementation.
pub struct PrefixRelayService {
    prefix: String,
    secure_channel_listener_flow_control_id: FlowControlId,
}
impl PrefixRelayService {
    pub async fn create(
        context: &Context,
        secure_channel_listener_flow_control_id: FlowControlId,
    ) -> Result<()> {
        // add the this worker as consumer for the secure channel listener
        let worker_address = Address::from_string(KAFKA_OUTLET_CONSUMERS);
        context.flow_controls().add_consumer(
            worker_address.clone(),
            &secure_channel_listener_flow_control_id,
        );

        let worker = Self {
            prefix: "consumer_".to_string(),
            secure_channel_listener_flow_control_id,
        };

        context
            .start_worker_with_access_control(
                worker_address,
                worker,
                AllowAll,
                AllowOnwardAddress(DefaultAddress::RELAY_SERVICE.into()),
            )
            .await
    }
}

#[ockam::worker]
impl Worker for PrefixRelayService {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // the payload is just a string
        let address = match msg.payload().get(1..) {
            Some(address) => match from_utf8(address) {
                Ok(v) => v.to_string(),
                Err(_e) => {
                    return Err(ockam_core::Error::new(
                        Origin::Application,
                        Kind::Invalid,
                        "invalid address",
                    ));
                }
            },
            None => {
                return Err(ockam_core::Error::new(
                    Origin::Application,
                    Kind::Invalid,
                    "invalid address",
                ));
            }
        };

        let new_address = format!("{}_{}", &self.prefix, address);

        debug!("prefix relay, renamed from {} to {}", address, new_address);

        let mut bytes = new_address.clone().into_bytes();
        let mut new_payload: Vec<u8> = vec![bytes.len() as u8];
        new_payload.append(&mut bytes);

        let mut message = msg.into_local_message();
        let transport_message = message.transport_mut();

        transport_message.onward_route.step()?;

        // prefix consumer_ to the address
        transport_message.payload = new_payload;

        ctx.forward(message).await?;

        // The new relay needs to be reachable by the default secure channel listener
        ctx.flow_controls().add_consumer(
            Address::from_string(new_address),
            &self.secure_channel_listener_flow_control_id,
        );

        Ok(())
    }
}
