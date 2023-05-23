use crate::kafka::ORCHESTRATOR_KAFKA_CONSUMERS;
use crate::DefaultAddress;
use core::str::from_utf8;
use ockam::{Any, Context, Result, Routed, Worker};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, AllowAll, AllowOnwardAddresses};

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

        debug!("msg.return_route(): {:?}", msg.return_route());
        debug!("msg.onward_route(): {:?}", msg.onward_route());
        debug!("msg.payload(): {:?}", msg.payload());

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

        let is_from_forwarding_service = msg.src_addr().address() == address;
        debug!("msg.src_addr(): {}", msg.src_addr());
        let new_address = if is_from_forwarding_service {
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

        // Insert my address at the beginning return_route if it's not a response from the
        // the forwarded service, in that case we want to be removed as the initiation is
        // finished
        // if !is_from_forwarding_service {
        //     transport_message
        //         .return_route
        //         .modify()
        //         .prepend(ctx.address());
        // }

        debug!("msg.return_route(): {:?}", transport_message.return_route);
        debug!("msg.onward_route(): {:?}", transport_message.onward_route);

        // Send the message on its onward_route
        ctx.forward(message).await
    }
}
