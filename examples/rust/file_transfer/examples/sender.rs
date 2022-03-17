// examples/sender.rs

use file_transfer::{FileData, FileDescription};
use ockam::{route, Context, Identity, TrustEveryonePolicy, Vault};
use ockam::{TcpTransport, TCP};

use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Debug, StructOpt)]
#[structopt(name = "sender", about = "An example of file transfer implemented with ockam.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Forwarding address
    #[structopt(short, long)]
    address: String,

    /// Sending chunk
    #[structopt(short, long, default_value = "8192")]
    chunk_size: usize,
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let opt = Opt::from_args();

    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Sender.
    let vault = Vault::create();

    // Create an Identity to represent Sender.
    let sender = Identity::create(&ctx, &vault).await?;

    // This program expects that the receiver has setup a forwarding address,
    // for his secure channel listener, on the Ockam node at 1.node.ockam.network:4000.
    //
    // Read this forwarding address for Receiver's secure channel listener from command line argument.
    let forwarding_address = opt.address.trim();

    // Combine the tcp address of the node and the forwarding_address to get a route
    // to Receiver's secure channel listener.
    let route_to_receiver_listener = route![(TCP, "1.node.ockam.network:4000"), forwarding_address, "listener"];

    // As Sender, connect to the Receiver's secure channel listener, and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Receiver.
    let channel = sender
        .create_secure_channel(route_to_receiver_listener, TrustEveryonePolicy)
        .await?;

    println!("\n[✓] End-to-end encrypted secure channel was established.\n");

    // Open file, send name and size to Receiver then send chunk of files.
    let mut file = File::open(&opt.input).await?;
    let metadata = file.metadata().await?;
    if !metadata.is_file() {
        anyhow::bail!("Can only transfer a file")
    }

    // Can safely unwrap the first time because we're sure we have a file because we opened it above
    //     and `file_name` returns None when the path ends with `..` which can't be a file
    // Can safely unwrap the second time because the path came from a String from the command line and thus should be a valid UTF8
    let filename = opt.input.file_name().unwrap().to_str().unwrap().to_owned();
    let descr = FileData::Description(FileDescription {
        name: filename,
        size: metadata.len() as usize,
    });

    ctx.send(route![channel.clone(), "receiver"], descr).await?;

    let mut buffer = vec![0u8; opt.chunk_size];
    loop {
        if let Ok(count) = file.read(&mut buffer).await {
            if count == 0 {
                break;
            }
            let data = FileData::Data(buffer[..count].to_vec());
            ctx.send(route![channel.clone(), "receiver"], data).await?;
        }
    }

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}
