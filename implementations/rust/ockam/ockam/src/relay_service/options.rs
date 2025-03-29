use crate::alloc::string::ToString;
use alloc::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl};
use ockam_identity::{Identifier, IdentitiesAttributes};

/// Trust Options for a Forwarding Service
pub struct RelayServiceOptions {
    pub(super) service_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) relays_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) consumer_service: Vec<FlowControlId>,
    pub(super) consumer_relay: Vec<FlowControlId>,
    pub(super) prefix: String,
    pub(super) authority_validation: Option<AuthorityValidation>,
    pub(super) aliases: Vec<Address>,
}

pub(super) struct AuthorityValidation {
    pub(super) authority: Identifier,
    pub(super) identities_attributes: Arc<IdentitiesAttributes>,
}

impl RelayServiceOptions {
    /// Default constructor without Access Control
    pub fn new() -> Self {
        Self {
            service_incoming_access_control: Arc::new(AllowAll),
            relays_incoming_access_control: Arc::new(AllowAll),
            consumer_service: vec![],
            consumer_relay: vec![],
            prefix: "".to_string(),
            authority_validation: None,
            aliases: vec![],
        }
    }

    /// Mark that this Relay service is a Consumer for to the given [`FlowControlId`]
    pub fn service_as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer_service.push(id.clone());

        self
    }

    /// Mark that spawned Relays are Consumers for to the given [`FlowControlId`]
    pub fn relay_as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer_relay.push(id.clone());

        self
    }

    /// Set Service Incoming Access Control
    pub fn with_service_incoming_access_control_impl(
        mut self,
        access_control: impl IncomingAccessControl,
    ) -> Self {
        self.service_incoming_access_control = Arc::new(access_control);
        self
    }

    /// Set Service Incoming Access Control
    pub fn with_service_incoming_access_control(
        mut self,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.service_incoming_access_control = access_control;
        self
    }

    /// Set spawned relays Incoming Access Control
    pub fn with_relays_incoming_access_control_impl(
        mut self,
        access_control: impl IncomingAccessControl,
    ) -> Self {
        self.relays_incoming_access_control = Arc::new(access_control);
        self
    }

    /// Set spawned relays Incoming Access Control
    pub fn with_relays_incoming_access_control(
        mut self,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.relays_incoming_access_control = access_control;
        self
    }

    /// Set Authority and IdentitiesAttributes
    pub fn authority(
        mut self,
        authority: Identifier,
        identities_attributes: Arc<IdentitiesAttributes>,
    ) -> Self {
        self.authority_validation = Some(AuthorityValidation {
            authority,
            identities_attributes,
        });
        self
    }

    /// Set Prefix for the Relay Service
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Add an alias for the Relay Service
    pub fn alias(mut self, alias: impl Into<Address>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    pub(super) fn setup_flow_control_for_relay_service(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer_service {
            flow_controls.add_consumer(address, id);
        }
    }

    pub(super) fn setup_flow_control_for_relay(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer_relay {
            flow_controls.add_consumer(address, id);
        }
    }
}

impl Default for RelayServiceOptions {
    fn default() -> Self {
        Self::new()
    }
}
