use crate::nodes::service::Alias;
use ockam::identity::IdentityIdentifier;
use ockam::remote::RemoteForwarderInfo;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Route};

#[derive(Default)]
pub(crate) struct SecureChannelRegistry {
    channels: Vec<SecureChannelInfo>,
}

impl SecureChannelRegistry {
    pub fn get_by_route(&self, route: &Route) -> Option<&SecureChannelInfo> {
        self.channels.iter().find(|&x| x.route() == route)
    }

    pub fn get_by_addr(&self, addr: &Address) -> Option<&SecureChannelInfo> {
        self.channels.iter().find(|&x| x.addr() == addr)
    }

    pub fn insert(
        &mut self,
        addr: Address,
        route: Route,
        sc_flow_control_id: FlowControlId,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    ) {
        self.channels.push(SecureChannelInfo::new(
            route,
            addr,
            sc_flow_control_id,
            authorized_identifiers,
        ))
    }

    pub fn remove_by_addr(&mut self, addr: &Address) {
        self.channels.retain(|x| x.addr() != addr)
    }

    pub fn list(&self) -> &[SecureChannelInfo] {
        &self.channels
    }
}

#[derive(Clone)]
pub struct SecureChannelInfo {
    // Target route of the channel
    route: Route,
    // Local address of the created channel
    addr: Address,
    flow_control_id: FlowControlId,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
}

impl SecureChannelInfo {
    pub fn new(
        route: Route,
        addr: Address,
        flow_control_id: FlowControlId,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    ) -> Self {
        Self {
            addr,
            route,
            flow_control_id,
            authorized_identifiers,
        }
    }

    pub fn route(&self) -> &Route {
        &self.route
    }

    pub fn addr(&self) -> &Address {
        &self.addr
    }

    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }

    pub fn authorized_identifiers(&self) -> Option<&Vec<IdentityIdentifier>> {
        self.authorized_identifiers.as_ref()
    }
}

#[derive(Clone)]
pub(crate) struct SecureChannelListenerInfo {
    addr: Address,
    flow_control_id: FlowControlId,
}

impl SecureChannelListenerInfo {
    pub fn new(addr: Address, flow_control_id: FlowControlId) -> Self {
        Self {
            addr,
            flow_control_id,
        }
    }

    pub fn addr(&self) -> &Address {
        &self.addr
    }

    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
}

#[derive(Default)]
pub(crate) struct IdentityServiceInfo {}

#[derive(Default)]
pub(crate) struct AuthenticatedServiceInfo {}

#[derive(Default)]
pub(crate) struct OktaIdentityProviderServiceInfo {}

#[derive(Default)]
pub(crate) struct UppercaseServiceInfo {}

#[derive(Default)]
pub(crate) struct EchoerServiceInfo {}

#[derive(Default)]
pub(crate) struct HopServiceInfo {}

#[derive(Default)]
pub(crate) struct VerifierServiceInfo {}

#[derive(Default)]
pub(crate) struct CredentialsServiceInfo {}

#[derive(Default)]
pub(crate) struct AuthenticatorServiceInfo {}

pub(crate) enum KafkaServiceKind {
    Consumer,
    Producer,
}

pub(crate) struct KafkaServiceInfo {
    kind: KafkaServiceKind,
}

impl KafkaServiceInfo {
    pub fn new(kind: KafkaServiceKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> &KafkaServiceKind {
        &self.kind
    }
}

#[derive(Clone)]
pub(crate) struct InletInfo {
    pub(crate) bind_addr: String,
    pub(crate) worker_addr: Address,
    pub(crate) outlet_route: Route,
}

impl InletInfo {
    pub(crate) fn new(
        bind_addr: &str,
        worker_addr: Option<&Address>,
        outlet_route: &Route,
    ) -> Self {
        let worker_addr = match worker_addr {
            Some(addr) => addr.clone(),
            None => Address::from_string(""),
        };
        Self {
            bind_addr: bind_addr.to_owned(),
            worker_addr,
            outlet_route: outlet_route.to_owned(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct OutletInfo {
    pub(crate) tcp_addr: String,
    pub(crate) worker_addr: Address,
}

impl OutletInfo {
    pub(crate) fn new(tcp_addr: &str, worker_addr: Option<&Address>) -> Self {
        let worker_addr = match worker_addr {
            Some(addr) => addr.clone(),
            None => Address::from_string(""),
        };
        Self {
            tcp_addr: tcp_addr.to_owned(),
            worker_addr,
        }
    }
}

#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) secure_channels: SecureChannelRegistry,
    pub(crate) secure_channel_listeners: BTreeMap<Address, SecureChannelListenerInfo>,
    pub(crate) identity_services: BTreeMap<Address, IdentityServiceInfo>,
    pub(crate) authenticated_services: BTreeMap<Address, AuthenticatedServiceInfo>,
    pub(crate) okta_identity_provider_services: BTreeMap<Address, OktaIdentityProviderServiceInfo>,
    pub(crate) uppercase_services: BTreeMap<Address, UppercaseServiceInfo>,
    pub(crate) echoer_services: BTreeMap<Address, EchoerServiceInfo>,
    pub(crate) kafka_services: BTreeMap<Address, KafkaServiceInfo>,
    pub(crate) hop_services: BTreeMap<Address, HopServiceInfo>,
    pub(crate) verifier_services: BTreeMap<Address, VerifierServiceInfo>,
    pub(crate) credentials_services: BTreeMap<Address, CredentialsServiceInfo>,
    #[cfg(feature = "direct-authenticator")]
    pub(crate) authenticator_service: BTreeMap<Address, AuthenticatorServiceInfo>,

    // FIXME: wow this is a terrible way to store data
    pub(crate) forwarders: BTreeMap<String, RemoteForwarderInfo>,
    pub(crate) inlets: BTreeMap<Alias, InletInfo>,
    pub(crate) outlets: BTreeMap<Alias, OutletInfo>,
}
