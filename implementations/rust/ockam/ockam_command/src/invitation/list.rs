use anyhow::anyhow;
use clap::Args;
use cli_table::{print_stdout, Cell, Style, Table};

use ockam::{route, Context, TcpTransport};
use ockam_api::cloud::MessagingClient;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::embedded_node;
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[clap(flatten)]
    identity_opts: IdentityOpts,
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
    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;
    let identifier = identity.identifier()?;

    let cloud_addr = ockam_api::multiaddr_to_route(&cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cloud_addr))?;
    let route = route![cloud_addr.to_string(), "invitations"];
    let mut api = MessagingClient::new(route, &ctx).await?;
    let invitations = api.list_invitations(identifier.key_id()).await?;
    let table = invitations
        .iter()
        .map(|i| {
            vec![
                format!("{:?}", i.id).cell(),
                format!("{:?}", i.space_id).cell(),
                format!("{:?}", i.project_id).cell(),
                format!("{:?}", i.inviter).cell(),
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
