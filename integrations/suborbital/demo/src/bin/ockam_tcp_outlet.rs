use clap::Parser;
use ockam::{Context, RemoteForwarder, Result, TcpTransport, TCP};
use ockam::{Identity, TrustEveryonePolicy, Vault};

#[derive(Parser)]
struct OutletArgs {
    /// Ockam's cloud node address
    cloud_addr: String,

    /// Alias that is used to identify Control Plane node
    alias: String,

    /// Address of tcp service running on Control Plane node that will receive
    /// connections from the Outlet
    outlet_target: String,

    /// Increase logging verbosity (enable debug logging)
    #[clap(short, long)]
    verbose: bool,

    /// Directory for persistance (for the vault).
    ///
    /// Will be created if missing. If no path specified, persistence is
    /// disabled.
    ///
    /// The vault file is created with `0o600` permissions (only owner can
    /// read/write), but note that it contains sensitive material.
    #[clap(long, parse(from_os_str))]
    data_dir: Option<std::path::PathBuf>,
}

fn main() {
    let args = OutletArgs::parse();
    suborbital_demo::init_logging(args.verbose);
    suborbital_demo::init_data_dir(&args.data_dir);
    let (ctx, executor) = ockam::start_node();
    let result = executor.execute(async move { node(&args, ctx).await });
    suborbital_demo::exit_with_result(args.verbose, result)
}

async fn node(args: &OutletArgs, ctx: Context) -> anyhow::Result<()> {
    let cloud_address = &args.cloud_addr;
    let alias = &args.alias;
    let outlet_target = &args.outlet_target;

    let tcp = TcpTransport::create(&ctx).await?;

    let vault = if let Some(dd) = &args.data_dir {
        Vault::from_path(dd.join("vault.json")).await?
    } else {
        Vault::create()
    };
    let mut e = Identity::create(&ctx, &vault).await?;

    e.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy)
        .await?;

    tcp.create_outlet("outlet", outlet_target).await?;

    let _ = RemoteForwarder::create_static(&ctx, (TCP, cloud_address), alias).await?;

    Ok(())
}
