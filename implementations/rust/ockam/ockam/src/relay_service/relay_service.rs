use crate::alloc::string::ToString;
use crate::relay_service::relay::Relay;
use crate::{Context, RelayServiceOptions};
use alloc::string::String;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{
    Address, DenyAll, Encodable, Mailbox, Mailboxes, Result, Routed, SecureChannelLocalInfo, Worker,
};
use ockam_node::WorkerBuilder;

/// Alias worker to register remote workers under local names.
///
/// To talk with this worker, you can use the
/// [`RemoteRelay`](crate::remote::RemoteRelay) which is a compatible client for this server.
#[non_exhaustive]
pub struct RelayService {
    options: RelayServiceOptions,
}

impl RelayService {
    /// Start a forwarding service
    pub async fn create(
        ctx: &Context,
        address: impl Into<Address>,
        options: RelayServiceOptions,
    ) -> Result<()> {
        let address = address.into();
        options.setup_flow_control_for_relay_service(ctx.flow_controls(), &address);

        let mut additional_mailboxes = vec![];
        for alias in &options.aliases {
            options.setup_flow_control_for_relay_service(ctx.flow_controls(), alias);
            additional_mailboxes.push(Mailbox::new(
                alias.clone(),
                options.service_incoming_access_control.clone(),
                Arc::new(DenyAll),
            ));
        }

        let service_incoming_access_control = options.service_incoming_access_control.clone();
        let s = Self { options };

        WorkerBuilder::new(s)
            .with_mailboxes(Mailboxes::new(
                Mailbox::new(
                    address.clone(),
                    service_incoming_access_control,
                    Arc::new(DenyAll),
                ),
                additional_mailboxes,
            ))
            .start(ctx)
            .await?;

        info!("Relay service started at {address}");

        Ok(())
    }
}

#[crate::worker]
impl Worker for RelayService {
    type Context = Context;
    type Message = String;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> Result<()> {
        let secure_channel_local_info =
            SecureChannelLocalInfo::find_info(message.local_message()).ok();

        let forward_route = message.return_route();
        let requested_relay_address = message.into_body()?;

        let requested_relay_name = if requested_relay_address == "register" {
            Address::random_tagged("Relay.service")
                .address()
                .to_string()
        } else {
            requested_relay_address
        };

        debug!(%requested_relay_name, "Relay creation request");

        // Verify the relay usage only when an authority is set, otherwise allow any relay name
        if let Some(authority_validation) = &self.options.authority_validation {
            if let Some(secure_channel_local_info) = secure_channel_local_info {
                let attributes = authority_validation
                    .identities_attributes
                    .get_attributes(
                        &secure_channel_local_info.their_identifier().into(),
                        &authority_validation.authority,
                    )
                    .await?;

                if let Some(attributes) = attributes {
                    let ockam_relay = attributes
                        .attrs()
                        .get("ockam-relay".as_bytes())
                        .and_then(|a| String::from_utf8(a.clone()).ok());

                    if let Some(ockam_relay) = ockam_relay {
                        match ockam_relay.as_str() {
                            "*" => {
                                // allow any relay name
                            }
                            allowed_name => {
                                if allowed_name != requested_relay_name {
                                    warn!(%allowed_name, %requested_relay_name, "Relay creation request not authorized, relay name does not match the attribute, dropping.");
                                    return Ok(());
                                }
                            }
                        }
                    } else {
                        warn!(%attributes, "Relay creation request not authorized, missing or invalid `ockam-relay` attribute, dropping.");
                        return Ok(());
                    }
                } else {
                    warn!("Relay creation request not authorized, missing `ockam-relay` attribute, no other attribute was found, dropping.");
                    return Ok(());
                }
            } else {
                warn!("Relay creation request not authenticated, dropping.");
                return Ok(());
            }
        }

        let final_relay_name = self.options.prefix.clone() + &requested_relay_name;
        let payload = final_relay_name.clone().encode()?;
        let final_relay_address = Address::from_string(final_relay_name);

        self.options
            .setup_flow_control_for_relay(ctx.flow_controls(), &final_relay_address);

        Relay::create(
            ctx,
            final_relay_address,
            forward_route,
            payload.to_vec(),
            self.options.relays_incoming_access_control.clone(),
        )
        .await?;

        Ok(())
    }
}
