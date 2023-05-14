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
        if msg.msg_addr() == self.addresses.heartbeat {
            // Heartbeat message, send registration message
            ctx.send_from_address(
                self.registration_route.clone(),
                self.registration_payload.clone(),
                self.addresses.main_remote.clone(),
            )
            .await?;

            if let Some(heartbeat) = &mut self.heartbeat {
                heartbeat.schedule(self.heartbeat_interval).await?;
            }

            Ok(())
        } else if msg.msg_addr() == self.addresses.main_remote {
            let return_route = msg.return_route();
            let mut message = msg.into_local_message();
            let transport_message = message.transport_mut();

            // Remove my address from the onward_route
            transport_message.onward_route.step()?;

            match transport_message.onward_route.next() {
                Err(_) => {
                    debug!("RemoteForwarder received service message");

                    let payload = Vec::<u8>::decode(&transport_message.payload)
                        .map_err(|_| OckamError::InvalidHubResponse)?;
                    let payload =
                        String::from_utf8(payload).map_err(|_| OckamError::InvalidHubResponse)?;
                    if payload != self.registration_payload {
                        return Err(OckamError::InvalidHubResponse.into());
                    }

                    if !self.completion_msg_sent {
                        info!("RemoteForwarder registered with route: {}", return_route);
                        let address = match return_route.recipient()?.to_string().strip_prefix("0#")
                        {
                            Some(addr) => addr.to_string(),
                            None => return Err(OckamError::InvalidHubResponse.into()),
                        };

                        ctx.send_from_address(
                            self.addresses.completion_callback.clone(),
                            RemoteForwarderInfo::new(
                                return_route,
                                address,
                                self.addresses.main_remote.clone(),
                                self.flow_control_id.clone(),
                            ),
                            self.addresses.main_remote.clone(),
                        )
                        .await?;

                        self.completion_msg_sent = true;
                    }

                    if let Some(heartbeat) = &mut self.heartbeat {
                        heartbeat.schedule(self.heartbeat_interval).await?;
                    }

                    Ok(())
                }
                Ok(next) if next == &self.addresses.main_remote => {
                    // Explicitly check that we don't forward to ourselves as this would somewhat
                    // overcome our outgoing access control, even though it shouldn't be possible
                    // to exploit it in any way
                    return Err(OckamError::UnknownForwarderNextHopAddress.into());
                }
                Ok(_) => {
                    // Forwarding the message
                    debug!("RemoteForwarder received payload message");

                    // Send the message on its onward_route
                    ctx.forward_from_address(message, self.addresses.main_internal.clone())
                        .await?;

                    // We received message from the other node, our registration is still alive, let's reset
                    // heartbeat timer
                    if let Some(heartbeat) = &mut self.heartbeat {
                        heartbeat.schedule(self.heartbeat_interval).await?;
                    }

                    Ok(())
                }
            }
        } else {
            Err(OckamError::UnknownForwarderDestinationAddress.into())
        }
    }
}
