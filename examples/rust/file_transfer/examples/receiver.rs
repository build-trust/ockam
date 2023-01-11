// examples/receiver.rs

use file_transfer::FileData;
use ockam::access_control::AllowAll;
use ockam::authenticated_storage::InMemoryStorage;
use ockam::{
    errcode::{Kind, Origin},
    identity::{Identity, SecureChannelRegistry, TrustEveryonePolicy},
    remote::RemoteForwarder,
    vault::Vault,
    Context, Error, Result, Routed, TcpTransport, Worker, TCP,
};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

#[derive(Default)]
struct FileReception {
    name: String,
    size: usize,
    written_size: usize,
    file: Option<tokio::fs::File>,
}

#[ockam::worker]
impl Worker for FileReception {
    type Context = Context;
    type Message = FileData;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Self::Message>) -> Result<()> {
        match msg.as_body() {
            FileData::Description(desc) => {
                self.name = desc.name.clone();
                self.size = desc.size;
                self.file = Some(
                    OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(&self.name)
                        .await
                        .map_err(|e| {
                            Error::new_without_cause(Origin::Application, Kind::Unknown).context("msg", e.to_string())
                        })?,
                )
            }
            FileData::Data(data) => {
                if self.written_size + data.len() > self.size {
                    return Err(Error::new_without_cause(Origin::Application, Kind::Unknown).context(
                        "msg",
                        format!(
                            "Received too many bytes already read: {}, received: {}, final size: {}",
                            self.written_size,
                            data.len(),
                            self.size
                        ),
                    ));
                }
                if let Some(file) = &mut self.file {
                    match file.write(data).await {
                        Ok(n) => {
                            self.written_size += n;
                            if self.written_size == self.size {
                                ctx.stop().await?;
                            }
                        }
                        Err(e) => {
                            return Err(Error::new(Origin::Application, Kind::Unknown, e));
                        }
                    }
                } else {
                    return Err(
                        Error::new_without_cause(Origin::Application, Kind::Unknown).context("msg", "file not opened")
                    );
                }
            }
            FileData::Quit => ctx.stop().await?,
        }

        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Receiver.
    let vault = Vault::create();

    // Create a SecureChannel registry.
    let registry = SecureChannelRegistry::new();

    // Create an Identity to represent Receiver.
    let receiver = Identity::create(&ctx, &vault).await?;

    // Create an AuthenticatedStorage to store info about Receiver's known Identities.
    let storage = InMemoryStorage::new();

    // Create a secure channel listener for Receiver that will wait for requests to
    // initiate an Authenticated Key Exchange.
    receiver
        .create_secure_channel_listener("listener", TrustEveryonePolicy, &storage, &registry)
        .await?;

    // The computer that is running this program is likely within a private network and
    // not accessible over the internet.
    //
    // To allow Sender and others to initiate an end-to-end secure channel with this program
    // we connect with 1.node.ockam.network:4000 as a TCP client and ask the forwarding
    // service on that node to create a forwarder for us.
    //
    // All messages that arrive at that forwarding address will be sent to this program
    // using the TCP connection we created as a client.
    let node_in_hub = (TCP, "1.node.ockam.network:4000");
    let forwarder = RemoteForwarder::create(&ctx, node_in_hub, AllowAll).await?;
    println!("\n[✓] RemoteForwarder was created on the node at: 1.node.ockam.network:4000");
    println!("Forwarding address for Receiver is:");
    println!("{}", forwarder.remote_address());

    // Start a worker, of type FileReception, at address "receiver".
    ctx.start_worker("receiver", FileReception::default(), AllowAll, AllowAll)
        .await?;

    // We won't call ctx.stop() here, this program will quit when the file will be entirely received
    Ok(())
}
