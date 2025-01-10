use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use ockam::udp::{RendezvousClient, UdpBindArguments, UdpBindOptions, UdpTransport, UDP};
use ockam_api::colors::color_primary;
use ockam_api::{fmt_log, DefaultAddress};
use ockam_core::{route, Address};
use ockam_node::Context;

/// Get my public UDP address from  Rendezvous service.
#[derive(Clone, Debug, Args)]
pub struct GetMyAddressCommand {
    /// The address of the Rendezvous service.
    #[arg(display_order = 900)]
    pub rendezvous_address: Option<String>,
}

#[async_trait]
impl Command for GetMyAddressCommand {
    const NAME: &'static str = "rendezvous get-my-address";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let udp = UdpTransport::create(ctx)?;
        let bind = udp
            .bind(
                UdpBindArguments::new().with_bind_address("0.0.0.0:0")?,
                UdpBindOptions::new(),
            )
            .await?;
        let rendezvous_address = self
            .rendezvous_address
            .map(|address| Address::new(UDP, address.into_bytes()))
            .unwrap_or(DefaultAddress::get_rendezvous_server_address());
        let client = RendezvousClient::new(
            &bind,
            route![rendezvous_address, DefaultAddress::RENDEZVOUS_SERVICE],
        );

        let my_address = client.get_my_address(ctx).await?;

        opts.terminal.write_line(fmt_log!(
            "Your public UDP address is {}",
            color_primary(my_address),
        ))?;

        Ok(())
    }
}
