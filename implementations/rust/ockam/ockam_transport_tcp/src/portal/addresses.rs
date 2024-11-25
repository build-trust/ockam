use core::fmt::Display;
use ockam_core::Address;

/// Enumerate all portal types
#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum PortalType {
    Inlet,
    Outlet,
    #[allow(unused)]
    PrivilegedInlet,
    #[allow(unused)]
    PrivilegedOutlet,
}

impl PortalType {
    pub fn str(&self) -> &'static str {
        match self {
            PortalType::Inlet | PortalType::PrivilegedInlet => "inlet",
            PortalType::Outlet | PortalType::PrivilegedOutlet => "outlet",
        }
    }

    pub fn is_privileged(&self) -> bool {
        match self {
            PortalType::Inlet | PortalType::Outlet => false,
            PortalType::PrivilegedInlet | PortalType::PrivilegedOutlet => true,
        }
    }
}

impl Display for PortalType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.str())
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
    pub(crate) fn generate(portal_type: PortalType) -> Self {
        let type_name = portal_type.str();
        let privileged_str = if portal_type.is_privileged() {
            "privileged"
        } else {
            "non_privileged"
        };
        let sender_internal = Address::random_tagged(&format!(
            "TcpPortalWorker.{}.{}.sender_internal",
            privileged_str, type_name
        ));
        let sender_remote = Address::random_tagged(&format!(
            "TcpPortalWorker.{}.{}.sender_remote",
            privileged_str, type_name
        ));
        let receiver_internal = Address::random_tagged(&format!(
            "TcpPortalRecvProcessor.{}.{}.receiver_internal",
            privileged_str, type_name
        ));
        let receiver_remote = Address::random_tagged(&format!(
            "TcpPortalRecvProcessor.{}.{}.receiver_remote",
            privileged_str, type_name
        ));

        Self {
            sender_internal,
            sender_remote,
            receiver_internal,
            receiver_remote,
        }
    }
}
