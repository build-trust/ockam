use async_trait::async_trait;
use clap::Args;
use console::Term;
use miette::miette;
use ockam_api::DefaultAddress;

use ockam_api::nodes::models::services::ServiceStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::output::Output;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::api::Request;
use ockam_node::Context;

use crate::tui::{PluralTerm, ShowCommandTui};
use crate::{node::NodeOpts, Command, CommandGlobalOpts};

/// Show a Kafka Inlet
#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// Kafka Inlet service address
    pub address: Option<String>,
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "kafka-inlet show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(ShowTui::run(ctx, opts, &self).await?)
    }
}

struct ShowTui<'a> {
    ctx: &'a Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: &'a ShowCommand,
}

impl<'a> ShowTui<'a> {
    pub async fn run(
        ctx: &'a Context,
        opts: CommandGlobalOpts,
        cmd: &'a ShowCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
        let tui = Self {
            ctx,
            opts,
            node,
            cmd,
        };
        tui.show().await
    }
}

#[async_trait]
impl<'a> ShowCommandTui for ShowTui<'a> {
    const ITEM_NAME: PluralTerm = PluralTerm::KafkaInlet;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.address.clone()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        Ok(self
            .cmd_arg_item_name()
            .unwrap_or(DefaultAddress::KAFKA_INLET.to_string()))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let inlets: Vec<ServiceStatus> = self
            .node
            .ask(
                self.ctx,
                Request::get(format!("/node/services/{}", DefaultAddress::KAFKA_INLET)),
            )
            .await?;
        let addresses = inlets.into_iter().map(|i| i.addr).collect();
        Ok(addresses)
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let inlets: Vec<ServiceStatus> = self
            .node
            .ask(
                self.ctx,
                Request::get(format!("/node/services/{}", DefaultAddress::KAFKA_INLET)),
            )
            .await?;
        let inlet = inlets
            .into_iter()
            .find(|i| i.addr == item_name)
            .ok_or_else(|| miette!("Kafka Inlet not found"))?;
        self.terminal()
            .stdout()
            .plain(inlet.item()?)
            .json_obj(&inlet)?
            .write_line()?;
        Ok(())
    }
}
