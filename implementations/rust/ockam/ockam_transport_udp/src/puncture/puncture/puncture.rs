use crate::puncture::puncture::Addresses;
use crate::puncture::UdpPunctureReceiverWorker;
use crate::{UdpBind, UdpPunctureOptions};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::Error;
use ockam_core::{Address, Result, Route};
use ockam_node::Context;
use tokio::sync::broadcast;

/// Individual puncture with a specified peer.
///
/// See [Wikipedia](https://en.wikipedia.org/wiki/UDP_hole_punching) and
/// ['Peer-to-Peer Communication Across Network Address Translators'](https://bford.info/pub/net/p2pnat/).
///
/// UDP and NAT Hole Punching are unreliable protocols. Expect send and receive
/// failures.
pub struct UdpPuncture {
    notify_puncture_open_receiver: broadcast::Receiver<Route>,
    notify_puncture_open_sender: broadcast::Sender<Route>,
    addresses: Addresses,
    flow_control_id: FlowControlId,
}

// TODO: PUNCTURE Allow to stop a puncture
// TODO: PUNCTURE make keepalives adjustable

impl UdpPuncture {
    pub(crate) async fn create(
        ctx: &Context,
        bind: &UdpBind,
        peer_udp_address: String,
        my_remote_address: Address,
        their_remote_address: Address,
        options: UdpPunctureOptions,
        // Will send messages to the UDP transport worker instead of the `UdpPunctureReceiverWorker`
        // on the other side, until we receive the first ping, which guarantees
        // that `UdpPunctureReceiverWorker` was started on the other side
        // See comments at the point of usage
        redirect_first_message_to_transport: bool,
    ) -> Result<UdpPuncture> {
        let flow_control_id = options.producer_flow_control_id();

        let addresses = Addresses::generate(my_remote_address);
        let (notify_puncture_open_sender, notify_puncture_open_receiver) = broadcast::channel(1);
        UdpPunctureReceiverWorker::create(
            ctx,
            bind,
            peer_udp_address,
            their_remote_address,
            addresses.clone(),
            notify_puncture_open_sender.clone(),
            options,
            redirect_first_message_to_transport,
        )
        .await?;

        Ok(UdpPuncture {
            notify_puncture_open_receiver,
            notify_puncture_open_sender,
            addresses,
            flow_control_id,
        })
    }

    /// Wait until puncture succeeds to the peer or a
    /// timeout. In case it's already open, will return on next pong message
    ///
    /// TODO: PUNCTURE optimize to return immediately if the puncture is open
    pub async fn wait_for_puncture(&mut self) -> Result<()> {
        self.notify_puncture_open_receiver
            .recv()
            .await
            .map_err(|_| {
                Error::new(
                    Origin::Transport,
                    Kind::Cancelled,
                    "UDP puncture won't be opened",
                )
            })?;

        Ok(())
    }

    /// Address of the Sender Worker
    pub fn sender_address(&self) -> Address {
        self.addresses.sender_address().clone()
    }

    /// Flow Control Id
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }

    /// Receiver that will receive a message with a Route when puncture is open, of empty route
    /// when closed
    // TODO: PUNCTURE send something more type safe and self-explanatory than a route
    //  The route value doesn't make much sense, we not the sender address from the very beginning
    //  Also, the empty route is a bad API
    pub fn notify_puncture_open_receiver(&self) -> broadcast::Receiver<Route> {
        self.notify_puncture_open_sender.subscribe()
    }
}
