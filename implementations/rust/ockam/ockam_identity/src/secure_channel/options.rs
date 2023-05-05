use crate::secure_channel::Addresses;
use crate::{TrustEveryonePolicy, TrustPolicy};
use core::fmt;
use core::fmt::Formatter;
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::{
    FlowControlId, FlowControlOutgoingAccessControl, FlowControlPolicy, FlowControls,
};
use ockam_core::{Address, OutgoingAccessControl, Result};

/// Trust options for a Secure Channel
pub struct SecureChannelOptions {
    pub(crate) producer_flow_control_id: FlowControlId,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
    pub(crate) timeout: Duration,
}

impl fmt::Debug for SecureChannelOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FlowId: {}", self.producer_flow_control_id)
    }
}

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
            timeout: Duration::from_secs(120),
        }
    }

    /// Set Trust Policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Freshly generated [`FlowControlId`]
    pub fn producer_flow_control_id(&self) -> FlowControlId {
        self.producer_flow_control_id.clone()
    }
}

impl SecureChannelOptions {
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

#[derive(Debug)]
pub(crate) struct CiphertextFlowControl {
    pub(crate) id: FlowControlId,
    pub(crate) policy: FlowControlPolicy,
}

/// Trust options for a Secure Channel Listener
pub struct SecureChannelListenerOptions {
    pub(crate) consumer_flow_control: Vec<CiphertextFlowControl>,
    pub(crate) spawner_flow_control_id: FlowControlId,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

impl fmt::Debug for SecureChannelListenerOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "SpawnerFlowId: {}", self.spawner_flow_control_id)
    }
}

impl SecureChannelListenerOptions {
    /// Mark spawned Secure Channel Decryptors as Producers for a given Spawner's [`FlowControlId`]
    /// NOTE: Spawned connections get fresh random [`FlowControlId`], however they are still marked
    /// with Spawner's [`FlowControlId`]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            consumer_flow_control: vec![],
            spawner_flow_control_id: FlowControls::generate_id(),
            trust_policy: Arc::new(TrustEveryonePolicy),
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
        self.consumer_flow_control.push(CiphertextFlowControl {
            id: flow_control_id.clone(),
            policy: flow_control_policy,
        });

        self
    }

    /// Set trust policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    /// Freshly generated [`FlowControlId`]
    pub fn spawner_flow_control_id(&self) -> FlowControlId {
        self.spawner_flow_control_id.clone()
    }
}

impl SecureChannelListenerOptions {
    pub(crate) fn setup_flow_control_for_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for consumer_flow_control in &self.consumer_flow_control {
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
