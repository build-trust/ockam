use clap::Args;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, multiaddr_to_route};
use crate::IdentityOpts;
use anyhow::anyhow;
use cli_table::{print_stdout, Cell, Style, Table};
use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[clap(display_order = 1101, long)]
    overwrite: bool,
}

impl<'a> From<&'a ListCommand> for IdentityOpts {
    fn from(other: &'a ListCommand) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

impl ListCommand {
    pub fn run(command: ListCommand, cloud_addr: MultiAddr) {
        embedded_node(list, (cloud_addr, command));
    }
}

async fn list(mut ctx: Context, args: (MultiAddr, ListCommand)) -> anyhow::Result<()> {
    let (cloud_addr, cmd) = args;
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&IdentityOpts::from(&cmd), &ctx).await?;

    let cloud_addr = multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![cloud_addr.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, &ctx).await?;
    let invitations = api.list_invitations(&identity.id.to_string()).await?;
    let table = invitations
        .iter()
        .map(|i| {
            vec![
                format!("{}", i.id).cell(),
                format!("{}", i.space_id).cell(),
                format!("{:?}", i.project_id).cell(),
                format!("{}", i.inviter).cell(),
                format!("{:?}", i.state).cell(),
            ]
        })
        .table()
        .title(vec![
            "Invitation ID".cell().bold(true),
            "Space".cell().bold(true),
            "Project".cell().bold(true),
            "Inviter".cell().bold(true),
            "State".cell().bold(true),
        ]);
    if let Err(e) = print_stdout(table) {
        eprintln!("failed to print node status: {}", e);
    }
    ctx.stop().await?;
    Ok(())
}
