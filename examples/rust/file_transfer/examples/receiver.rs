// examples/receiver.rs

use file_transfer::FileData;
use ockam::access_control::AllowAll;
use ockam::flow_control::FlowControlPolicy;
use ockam::identity::SecureChannelListenerOptions;
use ockam::remote::RemoteForwarderOptions;
use ockam::{
    errcode::{Kind, Origin},
    node, Context, Error, Result, Routed, TcpConnectionOptions, Worker,
};
use ockam_transport_tcp::TcpTransportExtension;
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
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity to represent Receiver.
    let receiver = node.create_identity().await?;

    let tcp_options = TcpConnectionOptions::new();
    let secure_channel_listener_options = SecureChannelListenerOptions::new().as_consumer(
        &tcp_options.producer_flow_control_id(),
        FlowControlPolicy::ProducerAllowMultiple,
    );

    node.flow_controls().add_consumer(
        "receiver",
        &secure_channel_listener_options.spawner_flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    // Create a secure channel listener for Receiver that will wait for requests to
    // initiate an Authenticated Key Exchange.
    node.create_secure_channel_listener(&receiver, "listener", secure_channel_listener_options)
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
    let node_in_hub = tcp.connect("1.node.ockam.network:4000", tcp_options).await?;
    let forwarder = node
        .create_forwarder(node_in_hub, RemoteForwarderOptions::new())
        .await?;
    println!("\n[✓] RemoteForwarder was created on the node at: 1.node.ockam.network:4000");
    println!("Forwarding address for Receiver is:");
    println!("{}", forwarder.remote_address());

    // Start a worker, of type FileReception, at address "receiver".
    node.start_worker("receiver", FileReception::default(), AllowAll, AllowAll)
        .await?;

    // We won't call ctx.stop() here, this program will quit when the file will be entirely received
    Ok(())
}
