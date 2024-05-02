use ockam_core::Address;

#[derive(Clone, Debug)]
/// Addresses used for UDP puncture
pub struct Addresses {
    /// Used to receive heartbeat messages
    heartbeat_address: Address,
    /// Used to receive message from the remote node
    remote_address: Address,
    /// Used to forward message from the node to the remote node
    sender_address: Address,
    /// Used to forward remote messages to addresses inside our node
    receiver_address: Address,
}

impl Addresses {
    pub(crate) fn generate(remote_address: Address) -> Self {
        let heartbeat_address = Address::random_tagged("UdpPunctureWorker.main");
        let sender_address = Address::random_tagged("UdpPunctureWorker.sender");
        let receiver_address = Address::random_tagged("UdpPunctureWorker.receiver");

        Self {
            heartbeat_address,
            remote_address,
            sender_address,
            receiver_address,
        }
    }

    /// Used to receive heartbeat messages
    pub fn heartbeat_address(&self) -> &Address {
        &self.heartbeat_address
    }

    /// Used to receive message from the remote node
    pub fn remote_address(&self) -> &Address {
        &self.remote_address
    }

    /// Used to forward message from the node to the remote node
    pub fn sender_address(&self) -> &Address {
        &self.sender_address
    }

    /// Used to forward remote messages to addresses inside our node
    pub fn receiver_address(&self) -> &Address {
        &self.receiver_address
    }
}
