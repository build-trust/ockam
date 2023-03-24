use ockam_core::Address;

/// Enumerate all portal types
#[derive(Debug, Clone)]
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
pub(super) struct Addresses {
    pub(super) internal: Address,
    pub(super) remote: Address,
    pub(super) receiver: Address,
}

impl Addresses {
    pub(super) fn generate(portal_type: PortalType) -> Self {
        let type_name = portal_type.str();
        let internal = Address::random_tagged(&format!("TcpPortalWorker.{}.internal", type_name));
        let remote = Address::random_tagged(&format!("TcpPortalWorker.{}.remote", type_name));
        let receiver = Address::random_tagged(&format!("TcpPortalRecvProcessor.{}", type_name));

        Self {
            internal,
            remote,
            receiver,
        }
    }
}
