use clap::Parser;
use ockam::{route, Context, Identity, Result, TcpTransport, TrustEveryonePolicy, Vault, TCP};
use suborbital_demo::exit_with_result;

#[derive(Parser)]
struct InletArgs {
    /// Directory for persistance (for the vault).
    ///
    /// Will be created if missing. If no path specified, persistence is
    /// disabled.
    ///
    /// The vault file is created at `$data_dir/vault.json` with `0o600`
    /// permissions, as it contains sensitive material.
    #[clap(long, parse(from_os_str))]
    data_dir: Option<std::path::PathBuf>,

    /// Ockam's cloud node address
    cloud_addr: String,
    /// Alias that is used to identify Control Plane node
    alias: String,
    /// Bind address for the inlet to listen on.
    inlet_address: String,
    /// Increase logging verbosity (enable debug logging)
    #[clap(short, long)]
    verbose: bool,
}

fn main() {
    let args = InletArgs::parse();
    suborbital_demo::init_logging(args.verbose);
    suborbital_demo::init_data_dir(&args.data_dir);
    let (ctx, executor) = ockam::start_node();
    let result = executor.execute(async move { node(&args, ctx).await });
    suborbital_demo::exit_with_result(args.verbose, result)
}

async fn node(args: &InletArgs, ctx: Context) -> anyhow::Result<()> {
    let cloud_address = &args.cloud_addr;
    let alias = &args.alias;
    let inlet_address = &args.inlet_address;

    let tcp = TcpTransport::create(&ctx).await?;

    let vault = if let Some(dd) = &args.data_dir {
        Vault::from_path(dd.join("vault.json")).await?
    } else {
        Vault::create()
    };
    let mut e = Identity::create(&ctx, &vault).await?;

    let channel = e
        .create_secure_channel(
            route![
                (TCP, cloud_address),
                format!("forward_to_{}", alias),
                "secure_channel_listener"
            ],
            TrustEveryonePolicy,
            // TODO: TrustIdentifierPolicy::new()
        )
        .await?;

    tcp.create_inlet(inlet_address, route![channel, "outlet"]).await?;

    Ok(())
}
