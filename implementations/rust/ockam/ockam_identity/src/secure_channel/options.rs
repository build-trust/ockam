use crate::secure_channel::Addresses;
use crate::{Credential, TrustContext, TrustEveryonePolicy, TrustPolicy};
use alloc::vec::Vec;
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{
    FlowControlId, FlowControlOutgoingAccessControl, FlowControlPolicy, FlowControls,
};
use ockam_core::{Address, OutgoingAccessControl, Result};

/// Trust options for a Secure Channel
pub struct SecureChannelOptions {
    pub(crate) producer_flow_control_id: FlowControlId,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
    pub(crate) trust_context: Option<TrustContext>,
    pub(crate) credentials: Vec<Credential>,
    pub(crate) timeout: Duration,
}

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) struct SecureChannelAccessControl {
    pub(crate) decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

impl SecureChannelOptions {
    /// Mark this Secure Channel Decryptor as a Producer with a random [`FlowControlId`]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            producer_flow_control_id: FlowControls::generate_id(),
            trust_policy: Arc::new(TrustEveryonePolicy),
            trust_context: None,
            credentials: vec![],
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Mark this Secure Channel Decryptor as a Producer for a given [`FlowControlId`]
    pub fn as_producer(flow_control_id: &FlowControlId) -> Self {
        Self {
            producer_flow_control_id: flow_control_id.clone(),
            trust_policy: Arc::new(TrustEveryonePolicy),
            trust_context: None,
            credentials: vec![],
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Sets a timeout different from the default one [`DEFAULT_TIMEOUT`]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Adds provided credentials
    pub fn with_credentials(mut self, credentials: Vec<Credential>) -> Self {
        self.credentials.extend(credentials);
        self
    }

    /// Adds a single credential
    pub fn with_credential(mut self, credential: Credential) -> Self {
        self.credentials.push(credential);
        self
    }

    /// Sets trust context
    pub fn with_trust_context(mut self, trust_context: TrustContext) -> Self {
        self.trust_context = Some(trust_context);
        self
    }

    /// Set Trust Policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    pub(crate) fn setup_flow_control(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
        next: &Address,
    ) -> Result<()> {
        if let Some(flow_control_id) = flow_controls
            .find_flow_control_with_producer_address(next)
            .map(|x| x.flow_control_id().clone())
        {
            // Allow a sender with corresponding flow_control_id send messages to this address
            flow_controls.add_consumer(
                addresses.decryptor_remote.clone(),
                &flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }

        flow_controls.add_producer(
            addresses.decryptor_internal.clone(),
            &self.producer_flow_control_id,
            None,
            vec![addresses.encryptor.clone()],
        );

        Ok(())
    }

    pub(crate) fn create_access_control(
        &self,
        flow_controls: &FlowControls,
    ) -> SecureChannelAccessControl {
        let ac = FlowControlOutgoingAccessControl::new(
            flow_controls,
            self.producer_flow_control_id.clone(),
            None,
        );

        SecureChannelAccessControl {
            decryptor_outgoing_access_control: Arc::new(ac),
        }
    }
}

pub(crate) struct CiphertextFlowControl {
    pub(crate) id: FlowControlId,
    pub(crate) policy: FlowControlPolicy,
}

/// Trust options for a Secure Channel Listener
pub struct SecureChannelListenerOptions {
    pub(crate) consumer_flow_control: Option<CiphertextFlowControl>,
    pub(crate) spawner_flow_control_id: FlowControlId,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
    pub(crate) trust_context: Option<TrustContext>,
    pub(crate) credentials: Vec<Credential>,
}

impl SecureChannelListenerOptions {
    /// Mark spawned Secure Channel Decryptors as Producers for a given Spawner's [`FlowControlId`]
    /// NOTE: Spawned connections get fresh random [`FlowControlId`], however they are still marked
    /// with Spawner's [`FlowControlId`]
    pub fn new(flow_control_id: &FlowControlId) -> Self {
        Self {
            consumer_flow_control: None,
            spawner_flow_control_id: flow_control_id.clone(),
            trust_policy: Arc::new(TrustEveryonePolicy),
            trust_context: None,
            credentials: vec![],
        }
    }

    /// Mark that this Secure Channel Listener is a Consumer for to the given [`FlowControlId`]
    /// Also, in this case spawned Secure Channels will be marked as Consumers with [`FlowControlId`]
    /// of the message that was used to create the Secure Channel
    pub fn as_consumer(
        mut self,
        flow_control_id: &FlowControlId,
        flow_control_policy: FlowControlPolicy,
    ) -> Self {
        self.consumer_flow_control = Some(CiphertextFlowControl {
            id: flow_control_id.clone(),
            policy: flow_control_policy,
        });

        self
    }

    /// Adds provided credentials
    pub fn with_credentials(mut self, credentials: Vec<Credential>) -> Self {
        self.credentials.extend(credentials);
        self
    }

    /// Adds a single credential
    pub fn with_credential(mut self, credential: Credential) -> Self {
        self.credentials.push(credential);
        self
    }

    /// Sets trust context
    pub fn with_trust_context(mut self, trust_context: TrustContext) -> Self {
        self.trust_context = Some(trust_context);
        self
    }

    /// Set trust policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    pub(crate) fn setup_flow_control_for_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        if let Some(consumer_flow_control) = &self.consumer_flow_control {
            flow_controls.add_consumer(
                address.clone(),
                &consumer_flow_control.id,
                consumer_flow_control.policy,
            );
        }

        flow_controls.add_spawner(address.clone(), &self.spawner_flow_control_id);
    }

    pub(crate) fn setup_flow_control_for_channel(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
        src_addr: &Address,
    ) -> FlowControlId {
        // Check if the Worker that send us this message is a Producer
        // If yes - decryptor will be added to that flow_control to be able to receive further messages
        // from that Producer
        if let Some(producer_flow_control_id) = flow_controls
            .get_flow_control_with_producer(src_addr)
            .map(|x| x.flow_control_id().clone())
        {
            // Allow a sender with corresponding flow_control_id send messages to this address
            flow_controls.add_consumer(
                addresses.decryptor_remote.clone(),
                &producer_flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }

        let flow_control_id = FlowControls::generate_id();
        flow_controls.add_producer(
            addresses.decryptor_internal.clone(),
            &flow_control_id,
            Some(&self.spawner_flow_control_id),
            vec![addresses.encryptor.clone()],
        );

        flow_control_id
    }

    pub(crate) fn create_access_control(
        &self,
        flow_controls: &FlowControls,
        flow_control_id: FlowControlId,
    ) -> SecureChannelAccessControl {
        let ac = FlowControlOutgoingAccessControl::new(
            flow_controls,
            flow_control_id,
            Some(self.spawner_flow_control_id.clone()),
        );

        SecureChannelAccessControl {
            decryptor_outgoing_access_control: Arc::new(ac),
        }
    }
}
