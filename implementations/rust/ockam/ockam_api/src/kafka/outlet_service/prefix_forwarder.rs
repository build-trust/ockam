use crate::kafka::ORCHESTRATOR_KAFKA_CONSUMERS;

use core::str::from_utf8;
use ockam::{Context, Result, Routed, Worker};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, AllowAll};

pub struct PrefixForwarderService {
    prefix: String,
}
impl PrefixForwarderService {
    pub async fn create(context: &Context) -> Result<()> {
        let worker = Self {
            prefix: "consumer_".to_string(),
        };

        context
            .start_worker(
                Address::from_string(ORCHESTRATOR_KAFKA_CONSUMERS),
                worker,
                AllowAll,
                AllowAll,
            )
            .await
    }
}

#[ockam::worker]
impl Worker for PrefixForwarderService {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // blindly forward if it comes from the forwarding service
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

        let new_address = if msg.src_addr().address() == address {
            address.replace(&format!("{}_", &self.prefix), "")
        } else {
            format!("{}_{}", &self.prefix, address)
        };

        debug!(
            "prefix forwarder, renamed from {} to {}",
            address, new_address
        );

        let mut bytes = new_address.into_bytes();
        let mut new_payload: Vec<u8> = vec![bytes.len() as u8];
        new_payload.append(&mut bytes);

        let mut message = msg.into_local_message();
        let transport_message = message.transport_mut();

        // Remove my address from the onward_route
        transport_message.onward_route.step()?;

        //prefix consumer_ to the address
        transport_message.payload = new_payload;

        // Send the message on its onward_route
        ctx.forward(message).await
    }
}
