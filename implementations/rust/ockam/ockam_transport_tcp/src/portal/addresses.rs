use ockam_core::Address;

/// Enumerate all portal types
#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum PortalType {
    Inlet,
    Outlet,
}

impl PortalType {
    pub fn str(&self) -> &'static str {
        match self {
            PortalType::Inlet => "inlet",
            PortalType::Outlet => "outlet",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Addresses {
    /// Used to receive messages from the corresponding receiver `receiver_internal` Address
    pub(crate) sender_internal: Address,
    /// Used to receive messages from the other side's Receiver
    pub(crate) sender_remote: Address,
    /// Used to send messages to the corresponding sender
    pub(crate) receiver_internal: Address,
    /// Used to send messages to the other side's Sender
    pub(crate) receiver_remote: Address,
}

impl Addresses {
    pub(super) fn generate(portal_type: PortalType) -> Self {
        let type_name = portal_type.str();
        let sender_internal =
            Address::random_tagged(&format!("TcpPortalWorker.{}.sender_internal", type_name));
        let sender_remote =
            Address::random_tagged(&format!("TcpPortalWorker.{}.sender_remote", type_name));
        let receiver_internal = Address::random_tagged(&format!(
            "TcpPortalRecvProcessor.{}.receiver_internal",
            type_name
        ));
        let receiver_remote = Address::random_tagged(&format!(
            "TcpPortalRecvProcessor.{}.receiver_remote",
            type_name
        ));

        Self {
            sender_internal,
            sender_remote,
            receiver_internal,
            receiver_remote,
        }
    }
}
