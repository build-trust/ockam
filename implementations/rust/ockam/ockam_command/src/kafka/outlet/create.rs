use async_trait::async_trait;

use clap::{command, Args};
use colorful::Colorful;
use miette::miette;
use serde::Serialize;
use std::fmt::Write;

use ockam::transport::SchemeHostnamePort;
use ockam::Context;
use ockam_abac::PolicyExpression;
use ockam_api::address::extract_address_value;
use ockam_api::colors::{color_primary, color_warn};
use ockam_api::nodes::models::services::StartKafkaOutletRequest;
use ockam_api::nodes::models::services::StartServiceRequest;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::output::Output;
use ockam_api::{fmt_log, fmt_ok, fmt_warn};
use ockam_core::api::Request;

use crate::node::util::initialize_default_node;
use crate::util::parsers::hostname_parser;
use crate::{
    docs,
    kafka::{kafka_default_outlet_addr, kafka_default_outlet_server},
    node::NodeOpts,
    Command, CommandGlobalOpts,
};

/// Create a new Kafka Outlet
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Address of your Kafka Outlet, which is part of a route used in other commands.
    /// This unique address identifies the Kafka Outlet worker on the Node on your local machine.
    /// Examples are `/service/my-outlet` or `my-outlet`.
    /// If not provided, `/service/kafka_outlet` will be used.
    /// You will need this address when creating a Kafka Inlet using `ockam kafka-inlet create`.
    #[arg(default_value_t = kafka_default_outlet_addr(), value_parser = extract_address_value)]
    pub name: String,

    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// Alternative to the <NAME> positional argument.
    /// Address of your Kafka Outlet, which is part of a route used in other commands.
    #[arg(long, id = "OUTLET_ADDRESS", visible_alias = "addr", default_value_t = kafka_default_outlet_addr(), value_parser = extract_address_value)]
    pub from: String,

    /// The address of the kafka bootstrap broker
    #[arg(long, visible_alias = "to", default_value_t = kafka_default_outlet_server(), value_parser = hostname_parser)]
    pub bootstrap_server: SchemeHostnamePort,

    /// [DEPRECATED] Use the `tls` scheme in the `--from` argument.
    #[arg(long, id = "BOOLEAN")]
    pub tls: bool,

    #[arg(help = docs::about("\
    Policy expression that will be used for access control to the Kafka Outlet. \
    If you don't provide it, the policy set for the \"tcp-outlet\" resource type will be used. \
    \n\nYou can check the fallback policy with `ockam policy show --resource-type tcp-outlet`."))]
    #[arg(long = "allow", id = "EXPRESSION")]
    pub policy_expression: Option<PolicyExpression>,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "kafka-outlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;

        let outlet = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating Kafka Outlet to bootstrap server {}...\n",
                    color_primary(cmd.bootstrap_server.to_string())
                ));
            }

            let payload = StartKafkaOutletRequest::new(
                cmd.bootstrap_server.clone().into(),
                cmd.tls || cmd.bootstrap_server.is_tls(),
                cmd.policy_expression,
            );
            let payload = StartServiceRequest::new(payload, &cmd.name);
            let req = Request::post("/node/services/kafka_outlet").body(payload);
            let node =
                BackgroundNodeClient::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
            node.tell(ctx, req)
                .await
                .map_err(|e| miette!("Failed to start Kafka Outlet: {e}"))?;

            KafkaOutletOutput {
                node_name: node.node_name(),
                bootstrap_server: cmd.bootstrap_server.to_string(),
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

impl CreateCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> miette::Result<Self> {
        if self.from != kafka_default_outlet_addr() {
            if self.name != kafka_default_outlet_addr() {
                opts.terminal.write_line(
                    fmt_warn!("The <NAME> argument is being overridden by the --from/--addr flag")
                        + &fmt_log!(
                            "Consider using either the <NAME> argument or the --from/--addr flag"
                        ),
                )?;
            }
            self.name = self.from.clone();
        }

        Ok(self)
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
