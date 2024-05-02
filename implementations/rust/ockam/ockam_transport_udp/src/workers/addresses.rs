use ockam_core::Address;

#[derive(Clone, Debug)]
pub(crate) struct Addresses {
    sender_address: Address,
    receiver_address: Address,
}

impl Addresses {
    pub(crate) fn generate() -> Self {
        let sender_address = Address::random_tagged("UdpSender");
        let receiver_address = Address::random_tagged("UdpReceiver");

        Self {
            sender_address,
            receiver_address,
        }
    }
    pub fn sender_address(&self) -> &Address {
        &self.sender_address
    }
    pub fn receiver_address(&self) -> &Address {
        &self.receiver_address
    }
}
