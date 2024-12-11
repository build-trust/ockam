use crate::messages::{UdpRoutingMessage, UdpTransportMessage};
use crate::workers::pending_messages::PeerPendingRoutingMessageStorage;
use ockam_core::compat::collections::HashMap;
use ockam_core::Result;
use std::net::SocketAddr;

/// Pending routing messages that we haven't yet assembled for all peers
/// TODO: Clearing everything for a socket after long inactivity would be nice
pub(crate) struct PendingRoutingMessageStorage {
    storage: HashMap<SocketAddr, PeerPendingRoutingMessageStorage>,
    max_pending_messages_per_peer: u16,
}

impl PendingRoutingMessageStorage {
    pub(crate) fn new(max_pending_messages_per_peer: u16) -> Self {
        Self {
            storage: Default::default(),
            max_pending_messages_per_peer,
        }
    }

    pub(crate) fn add_transport_message_and_try_assemble(
        &mut self,
        peer: SocketAddr,
        transport_message: UdpTransportMessage<'_>,
    ) -> Result<Option<UdpRoutingMessage<'static>>> {
        let routing_number = transport_message.routing_number;

        let peer_pending_messages = self.storage.entry(peer).or_insert_with(|| {
            PeerPendingRoutingMessageStorage::new(
                routing_number,
                self.max_pending_messages_per_peer,
            )
        });

        peer_pending_messages.add_transport_message_and_try_assemble(transport_message)
    }
}
