use async_trait::async_trait;

use clap::{command, Args};
use colorful::Colorful;
use miette::miette;
use serde::Serialize;
use std::fmt::Write;

use ockam::transport::HostnamePort;
use ockam::Context;
use ockam_abac::PolicyExpression;
use ockam_api::colors::{color_primary, color_warn};
use ockam_api::nodes::models::services::StartKafkaOutletRequest;
use ockam_api::nodes::models::services::StartServiceRequest;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::output::Output;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::api::Request;

use crate::node::util::initialize_default_node;
use crate::{
    kafka::{kafka_default_outlet_addr, kafka_default_outlet_server},
    node::NodeOpts,
    Command, CommandGlobalOpts,
};

/// Create a new Kafka Outlet
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// The local address of the service
    #[arg(long, default_value_t = kafka_default_outlet_addr())]
    pub addr: String,

    /// The address of the kafka bootstrap broker
    #[arg(long, default_value_t = kafka_default_outlet_server())]
    pub bootstrap_server: HostnamePort,

    /// If set, the outlet will establish a TLS connection over TCP
    #[arg(long, id = "BOOLEAN")]
    pub tls: bool,

    /// Policy expression that will be used for access control to the Kafka Outlet.
    /// If you don't provide it, the policy set for the "tcp-outlet" resource type will be used.
    ///
    /// You can check the fallback policy with `ockam policy show --resource-type tcp-outlet`.
    #[arg(hide = true, long = "allow", id = "EXPRESSION")]
    pub policy_expression: Option<PolicyExpression>,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "kafka-outlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;

        let outlet = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating Kafka Outlet to bootstrap server {}...\n",
                    color_primary(self.bootstrap_server.to_string())
                ));
            }

            let payload = StartKafkaOutletRequest::new(
                self.bootstrap_server.clone(),
                self.tls,
                self.policy_expression,
            );
            let payload = StartServiceRequest::new(payload, &self.addr);
            let req = Request::post("/node/services/kafka_outlet").body(payload);
            let node =
                BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
            node.tell(ctx, req)
                .await
                .map_err(|e| miette!("Failed to start Kafka Outlet: {e}"))?;

            KafkaOutletOutput {
                node_name: node.node_name(),
                bootstrap_server: self.bootstrap_server.to_string(),
            }
        };

        opts.terminal
            .stdout()
            .plain(outlet.item()?)
            .json_obj(outlet)?
            .write_line()?;

        Ok(())
    }
}

#[derive(Serialize)]
struct KafkaOutletOutput {
    node_name: String,
    bootstrap_server: String,
}

impl Output for KafkaOutletOutput {
    fn item(&self) -> ockam_api::Result<String> {
        let mut f = String::new();
        writeln!(
            f,
            "{}\n{}\n",
            fmt_ok!(
                "Created a new Kafka Outlet in the Node {}",
                color_primary(&self.node_name)
            ),
            fmt_log!(
                "bound to the bootstrap server at {}",
                color_primary(&self.bootstrap_server)
            ),
        )?;

        writeln!(
            f,
            "{}\n{}",
            fmt_log!(
                "{}",
                color_warn("Kafka clients v3.7.0 and earlier are supported")
            ),
            fmt_log!(
                "{}: {}",
                color_warn("You can find the version you have with"),
                color_primary("kafka-topics.sh --version")
            )
        )?;

        Ok(f)
    }
}
