use crate::util::{connect_to, exitcode, get_final_element};
use crate::CommandGlobalOpts;
use crate::{
    node::NodeOpts,
    util::{api, stop_node},
};
use clap::Args;
use ockam::{identity::PublicIdentity, vault::Vault, Context, NodeBuilder, Route};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::Status;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,
    #[clap(short, long, action)]
    full: bool,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) -> anyhow::Result<()> {
        let cfg = options.config;
        let node = get_final_element(&self.node_opts.api_node);
        if let Some(port) = cfg.try_get_node_port(node) {
            connect_to(port, self, show_identity);
        } else if let Some(default_identity) = cfg.get_default_identity() {
            if self.full {
                println!("{}", hex::encode(default_identity))
            } else {
                let vault = Vault::create();
                let (ctx, mut executor) =
                    NodeBuilder::without_access_control().no_logging().build();
                let _ = executor.execute(async move {
                    let r = PublicIdentity::import(&default_identity, &vault).await;
                    stop_node(ctx).await.unwrap();
                    match r {
                        Err(e) => {
                            eprintln!("An error occurred while importing identity: {}", e);
                            std::process::exit(exitcode::IOERR);
                        }
                        Ok(p) => println!("{}", p.identifier()),
                    }
                });
            }
        }

        Ok(())
    }
}

pub async fn show_identity(
    ctx: Context,
    cmd: ShowCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    if cmd.full {
        let resp: Vec<u8> = ctx
            .send_and_receive(
                base_route.modify().append(NODEMANAGER_ADDR),
                api::long_identity()?,
            )
            .await?;

        let (response, result) = api::parse_long_identity_response(&resp)?;

        match response.status() {
            Some(Status::Ok) => {
                println!("{}", hex::encode(result.identity.0.as_ref()))
            }
            _ => {
                eprintln!("An error occurred while exporting Identity",);
                std::process::exit(exitcode::IOERR);
            }
        }

        Ok(())
    } else {
        let resp: Vec<u8> = ctx
            .send_and_receive(
                base_route.modify().append(NODEMANAGER_ADDR),
                api::short_identity()?,
            )
            .await?;

        let (response, result) = api::parse_short_identity_response(&resp)?;

        match response.status() {
            Some(Status::Ok) => {
                println!("{}", result.identity_id)
            }
            _ => {
                eprintln!("An error occurred while getting Identity",);
                std::process::exit(exitcode::IOERR);
            }
        }

        Ok(())
    }
}
