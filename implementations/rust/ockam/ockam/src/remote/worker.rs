use crate::remote::{RemoteRelay, RemoteRelayInfo};
use crate::{Context, OckamError};
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
};
use ockam_core::{Any, Decodable, Result, Routed, Worker};
use tracing::{debug, info};

#[crate::worker]
impl Worker for RemoteRelay {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        debug!("RemoteRelay registration...");

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
            let mut local_message = msg.into_local_message();

            // Remove my address from the onward_route
            local_message = local_message.pop_front_onward_route()?;

            match local_message.onward_route().next() {
                Err(_) => {
                    debug!("RemoteRelay received service message");

                    let payload = String::decode(local_message.payload_ref())
                        .map_err(|_| OckamError::InvalidHubResponse)?;
                    // using ends_with() instead of == to allow for prefixes
                    if self.registration_payload != "register"
                        && !payload.ends_with(&self.registration_payload)
                    {
                        return Err(OckamError::InvalidHubResponse)?;
                    }

                    if !self.completion_msg_sent {
                        info!("RemoteRelay registered with route: {}", return_route);
                        let address = match return_route.recipient()?.to_string().strip_prefix("0#")
                        {
                            Some(addr) => addr.to_string(),
                            None => return Err(OckamError::InvalidHubResponse)?,
                        };

                        ctx.send_from_address(
                            self.addresses.completion_callback.clone(),
                            RemoteRelayInfo::new(
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
                    return Err(OckamError::UnknownForwarderNextHopAddress)?;
                }
                Ok(_) => {
                    // Forwarding the message
                    debug!("RemoteRelay received payload message");

                    // Send the message on its onward_route
                    ctx.forward_from_address(local_message, self.addresses.main_internal.clone())
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
            Err(OckamError::UnknownForwarderDestinationAddress)?
        }
    }
}
