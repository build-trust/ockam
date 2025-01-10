use miette::IntoDiagnostic;
use tracing::{error, info, instrument};

use crate::rendezvous::create::CreateCommand;
use crate::util::foreground_args::wait_for_exit_signal;
use crate::CommandGlobalOpts;
use ockam::transport::parse_socket_addr;
use ockam::udp::{RendezvousService, UdpBindArguments, UdpBindOptions, UdpTransport};
use ockam::Context;
use ockam_api::{DefaultAddress, RendezvousHealthcheck};

impl CreateCommand {
    #[instrument(skip_all)]
    pub(super) async fn foreground_mode(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        let udp_address = parse_socket_addr(&self.udp_address).into_diagnostic()?;

        info!(
            "Starting UDP Rendezvous service listening on {}",
            udp_address
        );

        RendezvousService::start(ctx, DefaultAddress::RENDEZVOUS_SERVICE).into_diagnostic()?;

        let udp = UdpTransport::create(ctx).into_diagnostic()?;
        let bind = udp
            .bind(
                UdpBindArguments::new().with_bind_socket_address(udp_address),
                UdpBindOptions::new(),
            )
            .await
            .into_diagnostic()?;

        ctx.flow_controls().add_consumer(
            &DefaultAddress::RENDEZVOUS_SERVICE.into(),
            bind.flow_control_id(),
        );

        let mut healthcheck =
            RendezvousHealthcheck::create(&self.healthcheck_address, &udp, udp_address)
                .into_diagnostic()?;
        healthcheck.start().await.into_diagnostic()?;

        wait_for_exit_signal(
            &self.foreground_args,
            &opts,
            "To exit and stop the Rendezvous Server, please press Ctrl+C\n",
        )
        .await?;

        // Clean up and exit
        if let Err(err) = healthcheck.stop().await {
            error!("Error while stopping healthcheck: {}", err);
        }
        Ok(())
    }
}
