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
    pub(crate) key_exchange_only: bool,
}

impl fmt::Debug for SecureChannelOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FlowId: {}", self.flow_control_id)
    }
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
            key_exchange_only: false,
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
        if self.key_exchange_only {
            return Err(IdentityError::CredentialRetrieverCreatorAlreadySet.into());
        }
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

    /// The secure channel will be used to exchange key only.
    /// In this mode, the secure channel cannot be used to exchange messages, and key rotation
    /// is disabled.
    ///
    /// Conflicts with [`with_credential_retriever_creator`] and [`with_credential`]
    pub fn key_exchange_only(mut self) -> Result<Self> {
        if self.credential_retriever_creator.is_some() {
            return Err(IdentityError::CredentialRetrieverCreatorAlreadySet.into());
        }
        self.key_exchange_only = true;
        Ok(self)
    }
}

impl SecureChannelOptions {
    pub(crate) fn setup_flow_control_producer(
        flow_control_id: &FlowControlId,
        flow_controls: &FlowControls,
        addresses: &Addresses,
    ) {
        flow_controls.add_producer(
            addresses.decryptor_internal.clone(),
            flow_control_id,
            None,
            vec![addresses.encryptor.clone()],
        );
    }

    pub(crate) fn setup_flow_control_consumer(
        flow_controls: &FlowControls,
        addresses: &Addresses,
        next: &Address,
    ) {
        if let Some(flow_control_id) = flow_controls
            .find_flow_control_with_producer_address(next)
            .map(|x| x.flow_control_id().clone())
        {
            // Allow a sender with corresponding flow_control_id send messages to this address
            flow_controls.add_consumer(addresses.decryptor_remote.clone(), &flow_control_id);
        }
    }

    pub(crate) fn setup_flow_control(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
        next: &Address,
    ) {
        Self::setup_flow_control_consumer(flow_controls, addresses, next);
        Self::setup_flow_control_producer(&self.flow_control_id, flow_controls, addresses);
    }

    pub(crate) fn create_decryptor_outgoing_access_control(
        &self,
        flow_controls: &FlowControls,
    ) -> Arc<dyn OutgoingAccessControl> {
        let ac = FlowControlOutgoingAccessControl::new(
            flow_controls,
            self.flow_control_id.clone(),
            None,
        );

        Arc::new(ac)
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
    pub(crate) key_exchange_only: bool,
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
            key_exchange_only: false,
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
        if self.key_exchange_only {
            return Err(IdentityError::CredentialRetrieverCreatorAlreadySet.into());
        }
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

    /// The listener will be used to exchange key only.
    /// In this mode, the secure channel cannot be used to exchange messages, and key rotation
    /// is disabled.
    ///
    /// Conflicts with [`with_credential_retriever_creator`] and [`with_credential`]
    pub fn key_exchange_only(mut self) -> Result<Self> {
        if self.credential_retriever_creator.is_some() {
            return Err(IdentityError::CredentialRetrieverCreatorAlreadySet.into());
        }
        self.key_exchange_only = true;
        Ok(self)
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
        listener_address: &Address,
        addresses: &Addresses,
    ) -> FlowControlId {
        // Add decryptor as consumer for the same ids as the listener, so that even if the initiator
        // updates the route - decryptor is still reachable
        for id in flow_controls.get_flow_control_ids_for_consumer(listener_address) {
            flow_controls.add_consumer(addresses.decryptor_remote.clone(), &id);
        }

        // TODO: What if we added a listener as a consumer for new FlowControlIds, should existing
        //  secure channels be accessible through these new ids?
        //  Consider following flow:
        //   1. You have a secure channel listener listener1 accessible from a tcp listener tcp1.
        //   2. A secure channel sc1 is established
        //   3. You start TcpListener tcp2
        //   4. You make existing listener1 accessible from tcp2
        //  Should sc1 now be accessible from tcp2? In current implementation it won't be. That's something to consider

        let flow_control_id = FlowControls::generate_flow_control_id();
        flow_controls.add_producer(
            addresses.decryptor_internal.clone(),
            &flow_control_id,
            Some(&self.flow_control_id),
            vec![addresses.encryptor.clone()],
        );

        flow_control_id
    }

    pub(crate) fn create_decryptor_outgoing_access_control(
        &self,
        flow_controls: &FlowControls,
        flow_control_id: FlowControlId,
    ) -> Arc<dyn OutgoingAccessControl> {
        let ac = FlowControlOutgoingAccessControl::new(
            flow_controls,
            flow_control_id,
            Some(self.flow_control_id.clone()),
        );

        Arc::new(ac)
    }
}
