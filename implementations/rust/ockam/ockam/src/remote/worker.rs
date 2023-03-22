use crate::remote::{RemoteForwarder, RemoteForwarderInfo};
use crate::{Context, OckamError};
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{Any, Decodable, Result, Routed, Worker};
use tracing::{debug, info};

#[crate::worker]
impl Worker for RemoteForwarder {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        debug!("RemoteForwarder registration...");

        ctx.send_from_address(
            self.registration_route.clone(),
            self.registration_payload.clone(),
            self.addresses.main_remote.clone(),
        )
        .await?;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // Heartbeat message, send registration message
        if msg.msg_addr() == self.addresses.heartbeat {
            ctx.send_from_address(
                self.registration_route.clone(),
                self.registration_payload.clone(),
                self.addresses.main_remote.clone(),
            )
            .await?;

            if let Some(heartbeat) = &mut self.heartbeat {
                heartbeat.schedule(self.heartbeat_interval).await?;
            }

            return Ok(());
        }

        // FIXME: @ac check that return address is the same
        // We are the final recipient of the message because it's registration response for our Worker
        if msg.onward_route().recipient()? == self.addresses.main_remote {
            debug!("RemoteForwarder received service message");

            let payload =
                Vec::<u8>::decode(msg.payload()).map_err(|_| OckamError::InvalidHubResponse)?;
            let payload = String::from_utf8(payload).map_err(|_| OckamError::InvalidHubResponse)?;
            if payload != self.registration_payload {
                return Err(OckamError::InvalidHubResponse.into());
            }

            if !self.completion_msg_sent {
                let route = msg.return_route();

                info!("RemoteForwarder registered with route: {}", route);
                let address = match route.clone().recipient()?.to_string().strip_prefix("0#") {
                    Some(addr) => addr.to_string(),
                    None => return Err(OckamError::InvalidHubResponse.into()),
                };

                ctx.send_from_address(
                    self.addresses.completion_callback.clone(),
                    RemoteForwarderInfo {
                        forwarding_route: route,
                        remote_address: address,
                        worker_address: ctx.address(),
                    },
                    self.addresses.main_remote.clone(),
                )
                .await?;

                self.completion_msg_sent = true;
            }

            if let Some(heartbeat) = &mut self.heartbeat {
                heartbeat.schedule(self.heartbeat_interval).await?;
            }
        } else {
            debug!("RemoteForwarder received payload message");

            let mut message = msg.into_local_message();
            let transport_message = message.transport_mut();

            // Remove my address from the onward_route
            transport_message.onward_route.step()?;

            // Send the message on its onward_route
            ctx.forward_from_address(message, self.addresses.main_internal.clone())
                .await?;

            // We received message from the other node, our registration is still alive, let's reset
            // heartbeat timer
            if let Some(heartbeat) = &mut self.heartbeat {
                heartbeat.schedule(self.heartbeat_interval).await?;
            }
        }

        Ok(())
    }
}
