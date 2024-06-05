// examples/sender.rs

use file_transfer::{FileData, FileDescription};
use ockam::errcode::{Kind, Origin};
use ockam::identity::SecureChannelOptions;
use ockam::tcp::{TcpConnectionOptions, TcpTransportExtension};
use ockam::{node, route, Context, Error, Result};

use std::path::PathBuf;

use clap::Parser;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Debug, Parser)]
#[command(name = "sender", about = "An example of file transfer implemented with ockam.")]
struct Sender {
    /// Input file
    input: PathBuf,

    /// Forwarding address
    #[arg(short, long)]
    address: String,

    /// Sending chunk
    #[arg(short, long, default_value = "8192")]
    chunk_size: usize,
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let opt = Sender::parse();

    let node = node(ctx).await?;
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity to represent Sender.
    let sender = node.create_identity().await?;

    // This program expects that the receiver has setup a forwarding address,
    // for his secure channel listener, on the Ockam node at 1.node.ockam.network:4000.
    //
    // Read this forwarding address for Receiver's secure channel listener from command line argument.
    let forwarding_address = opt.address.trim();

    // Connect to the cloud node over TCP
    let node_in_hub = tcp
        .connect("1.node.ockam.network:4000", TcpConnectionOptions::new())
        .await?;

    // Combine the tcp address of the cloud node and the forwarding_address to get a route
    // to Receiver's secure channel listener.
    let route_to_receiver_listener = route![node_in_hub, forwarding_address, "listener"];

    // As Sender, connect to the Receiver's secure channel listener, and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Receiver.
    let channel = node
        .create_secure_channel(&sender, route_to_receiver_listener, SecureChannelOptions::new())
        .await?;

    println!("\n[✓] End-to-end encrypted secure channel was established.\n");

    // Open file, send name and size to Receiver then send chunk of files.
    let mut file = File::open(&opt.input)
        .await
        .map_err(|e| Error::new(Origin::Executor, Kind::Unknown, e))?;
    let metadata = file
        .metadata()
        .await
        .map_err(|e| Error::new(Origin::Executor, Kind::Unknown, e))?;
    if !metadata.is_file() {
        return Err(Error::new(Origin::Executor, Kind::Unknown, "not a file"));
    }

    // Can safely unwrap the first time because we're sure we have a file because we opened it above
    //     and `file_name` returns None when the path ends with `..` which can't be a file
    // Can safely unwrap the second time because the path came from a String from the command line and thus should be a valid UTF8
    let filename = opt.input.file_name().unwrap().to_str().unwrap().to_owned();
    let descr = FileData::Description(FileDescription {
        name: filename,
        size: metadata.len() as usize,
    });

    node.send(route![channel.clone(), "receiver"], descr).await?;

    let mut buffer = vec![0u8; opt.chunk_size];
    loop {
        if let Ok(count) = file.read(&mut buffer).await {
            if count == 0 {
                break;
            }
            let data = FileData::Data(buffer[..count].to_vec());
            node.send(route![channel.clone(), "receiver"], data).await?;
        }
    }

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}
