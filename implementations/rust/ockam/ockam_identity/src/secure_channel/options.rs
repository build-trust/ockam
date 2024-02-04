use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::{FlowControlId, FlowControlOutgoingAccessControl, FlowControls};
use ockam_core::{Address, OutgoingAccessControl, Result};

use crate::models::CredentialAndPurposeKey;
use crate::secure_channel::Addresses;
use crate::{
    CredentialRetrieverCreator, Identifier, IdentityError, MemoryCredentialRetrieverCreator,
    TrustEveryonePolicy, TrustPolicy,
};

use core::fmt;
use core::fmt::Formatter;
use core::time::Duration;

/// This is the default timeout for creating a secure channel
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Trust options for a Secure Channel
pub struct SecureChannelOptions {
    pub(crate) flow_control_id: FlowControlId,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
    // To verify other party's credentials
    pub(crate) authority: Option<Identifier>,
    // To obtain our credentials
    pub(crate) credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
    pub(crate) timeout: Duration,
}

impl fmt::Debug for SecureChannelOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FlowId: {}", self.flow_control_id)
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
            flow_control_id: FlowControls::generate_flow_control_id(),
            trust_policy: Arc::new(TrustEveryonePolicy),
            authority: None,
            credential_retriever_creator: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Sets a timeout different from the default one [`DEFAULT_TIMEOUT`]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set [`CredentialRetrieverCreator`]
    pub fn with_credential_retriever_creator(
        mut self,
        credential_retriever_creator: Arc<dyn CredentialRetrieverCreator>,
    ) -> Result<Self> {
        if self.credential_retriever_creator.is_some() {
            return Err(IdentityError::CredentialRetrieverCreatorAlreadySet.into());
        }
        self.credential_retriever_creator = Some(credential_retriever_creator);
        Ok(self)
    }

    /// Set credential
    pub fn with_credential(self, credential: CredentialAndPurposeKey) -> Result<Self> {
        self.with_credential_retriever_creator(Arc::new(MemoryCredentialRetrieverCreator::new(
            credential,
        )))
    }

    /// Sets Trusted Authority
    pub fn with_authority(mut self, authority: Identifier) -> Self {
        self.authority = Some(authority);
        self
    }

    /// Set Trust Policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    /// Freshly generated [`FlowControlId`]
    pub fn producer_flow_control_id(&self) -> FlowControlId {
        self.flow_control_id.clone()
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
            flow_controls.add_consumer(addresses.decryptor_remote.clone(), &flow_control_id);
        }

        flow_controls.add_producer(
            addresses.decryptor_internal.clone(),
            &self.flow_control_id,
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
            self.flow_control_id.clone(),
            None,
        );

        SecureChannelAccessControl {
            decryptor_outgoing_access_control: Arc::new(ac),
        }
    }
}

/// Trust options for a Secure Channel Listener
pub struct SecureChannelListenerOptions {
    pub(crate) consumer: Vec<FlowControlId>,
    pub(crate) flow_control_id: FlowControlId,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
    // To verify other party's credentials
    pub(crate) authority: Option<Identifier>,
    // To obtain our credentials
    pub(crate) credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
}

impl fmt::Debug for SecureChannelListenerOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "SpawnerFlowId: {}", self.flow_control_id)
    }
}

impl SecureChannelListenerOptions {
    /// Mark spawned Secure Channel Decryptors as Producers for a given Spawner's [`FlowControlId`]
    /// NOTE: Spawned connections get fresh random [`FlowControlId`], however they are still marked
    /// with Spawner's [`FlowControlId`]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            consumer: vec![],
            flow_control_id: FlowControls::generate_flow_control_id(),
            trust_policy: Arc::new(TrustEveryonePolicy),
            authority: None,
            credential_retriever_creator: None,
        }
    }

    /// Mark that this Secure Channel Listener is a Consumer for to the given [`FlowControlId`]
    /// Also, in this case spawned Secure Channels will be marked as Consumers with [`FlowControlId`]
    /// of the message that was used to create the Secure Channel
    pub fn as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer.push(id.clone());

        self
    }

    /// Set [`CredentialRetrieverCreator`]
    pub fn with_credential_retriever_creator(
        mut self,
        credential_retriever_creator: Arc<dyn CredentialRetrieverCreator>,
    ) -> Result<Self> {
        if self.credential_retriever_creator.is_some() {
            return Err(IdentityError::CredentialRetrieverCreatorAlreadySet.into());
        }
        self.credential_retriever_creator = Some(credential_retriever_creator);
        Ok(self)
    }

    /// Set credential
    pub fn with_credential(self, credential: CredentialAndPurposeKey) -> Result<Self> {
        self.with_credential_retriever_creator(Arc::new(MemoryCredentialRetrieverCreator::new(
            credential,
        )))
    }

    /// Sets Trusted Authority
    pub fn with_authority(mut self, authority: Identifier) -> Self {
        self.authority = Some(authority);
        self
    }

    /// Set trust policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    /// Freshly generated [`FlowControlId`]
    pub fn spawner_flow_control_id(&self) -> FlowControlId {
        self.flow_control_id.clone()
    }
}

impl SecureChannelListenerOptions {
    pub(crate) fn setup_flow_control_for_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer {
            flow_controls.add_consumer(address.clone(), id);
        }

        flow_controls.add_spawner(address.clone(), &self.flow_control_id);
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
            );
        }

        let flow_control_id = FlowControls::generate_flow_control_id();
        flow_controls.add_producer(
            addresses.decryptor_internal.clone(),
            &flow_control_id,
            Some(&self.flow_control_id),
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
            Some(self.flow_control_id.clone()),
        );

        SecureChannelAccessControl {
            decryptor_outgoing_access_control: Arc::new(ac),
        }
    }
}
