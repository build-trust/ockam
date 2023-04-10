use crate::channel::addresses::Addresses;
use crate::credential::{Credential, CredentialExchangeMode};
use crate::error::IdentityError;
use crate::{TrustContext, TrustEveryonePolicy, TrustPolicy};
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{
    FlowControlId, FlowControlOutgoingAccessControl, FlowControlPolicy, FlowControls,
};
use ockam_core::{Address, AllowAll, OutgoingAccessControl, Result};

/// Trust options for a Secure Channel
pub struct SecureChannelOptions {
    pub(crate) consumer_flow_control: Option<FlowControls>,
    pub(crate) producer_flow_control: Option<(FlowControls, FlowControlId)>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
    pub(crate) trust_context: Option<TrustContext>,
    pub(crate) credential_exchange_mode: CredentialExchangeMode,
    pub(crate) credential: Option<Credential>,
}

pub(crate) struct SecureChannelAccessControl {
    pub(crate) decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

impl SecureChannelOptions {
    /// This constructor is insecure, because outgoing messages from such channels will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            consumer_flow_control: None,
            producer_flow_control: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
            trust_context: None,
            credential_exchange_mode: CredentialExchangeMode::None,
            credential: None,
        }
    }

    /// Mark this Secure Channel Decryptor as a Consumer. [`FlowControlId`] will be deducted from
    /// next hop of onward_route automatically
    pub fn as_consumer(mut self, flow_controls: &FlowControls) -> Self {
        self.consumer_flow_control = Some(flow_controls.clone());
        self
    }

    /// Mark this Secure Channel Decryptor as a Producer for a given [`FlowControlId`]
    pub fn as_producer(flow_controls: &FlowControls, flow_control_id: &FlowControlId) -> Self {
        Self {
            consumer_flow_control: None,
            producer_flow_control: Some((flow_controls.clone(), flow_control_id.clone())),
            trust_policy: Arc::new(TrustEveryonePolicy),
            trust_context: None,
            credential_exchange_mode: CredentialExchangeMode::None,
            credential: None,
        }
    }

    /// Set Trust Policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    /// Set Credential
    pub fn with_credential(mut self, credential: Credential) -> Self {
        self.credential = Some(credential);
        self
    }

    /// Set Credential Exchange Mode. Default is [`CredentialExchangeMode::None`]
    pub fn with_credential_exchange_mode(mut self, mode: CredentialExchangeMode) -> Self {
        self.credential_exchange_mode = mode;
        self
    }

    /// Set Trust Context.
    pub fn with_trust_context(mut self, trust_context: TrustContext) -> Self {
        self.trust_context = Some(trust_context);
        self
    }

    pub(crate) fn setup_flow_control(&self, addresses: &Addresses, next: &Address) -> Result<()> {
        match &self.consumer_flow_control {
            Some(flow_controls) => {
                if let Some(flow_control_id) = flow_controls
                    .find_flow_control_with_producer_address(next)
                    .map(|x| x.flow_control_id().clone())
                {
                    // Allow a sender with corresponding flow_control_id send messages to this address
                    flow_controls.add_consumer(
                        &addresses.decryptor_remote,
                        &flow_control_id,
                        FlowControlPolicy::ProducerAllowMultiple,
                    );
                }
            }
            None => {}
        }

        if let Some((flow_controls, flow_control_id)) = &self.producer_flow_control {
            flow_controls.add_producer(
                &addresses.decryptor_internal,
                flow_control_id,
                None,
                vec![addresses.encryptor.clone()],
            );
        }

        Ok(())
    }

    pub(crate) fn create_access_control(&self) -> SecureChannelAccessControl {
        match &self.producer_flow_control {
            Some((flow_controls, flow_control_id)) => {
                let ac = FlowControlOutgoingAccessControl::new(
                    flow_controls.clone(),
                    flow_control_id.clone(),
                    None,
                );

                SecureChannelAccessControl {
                    decryptor_outgoing_access_control: Arc::new(ac),
                }
            }
            None => SecureChannelAccessControl {
                decryptor_outgoing_access_control: Arc::new(AllowAll),
            },
        }
    }
}

pub(crate) struct CiphertextFlowControlInfo {
    pub(crate) flow_control_id: FlowControlId,
    pub(crate) flow_control_policy: FlowControlPolicy,
}

pub(crate) struct CiphertextFlowControl {
    pub(crate) flow_controls: FlowControls,
    pub(crate) info: Option<CiphertextFlowControlInfo>,
}

/// Trust options for a Secure Channel Listener
pub struct SecureChannelListenerOptions {
    pub(crate) consumer_flow_control: Option<CiphertextFlowControl>,
    pub(crate) channels_producer_flow_control: Option<(FlowControls, FlowControlId)>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
    pub(crate) trust_context: Option<TrustContext>,
    pub(crate) credential: Option<Credential>,
}

impl SecureChannelListenerOptions {
    /// This constructor is insecure, because outgoing messages from such channels will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            consumer_flow_control: None,
            channels_producer_flow_control: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
            trust_context: None,
            credential: None,
        }
    }

    /// Mark that this Secure Channel Listener is a Consumer for to the given [`FlowControlId`]
    /// Also, in this case spawned Secure Channels will be marked as Consumers with [`FlowControlId`]
    /// of the message that was used to create the Secure Channel
    pub fn as_consumer_with_flow_control_id(
        mut self,
        flow_controls: &FlowControls,
        flow_control_id: &FlowControlId,
        flow_control_policy: FlowControlPolicy,
    ) -> Self {
        self.consumer_flow_control = Some(CiphertextFlowControl {
            flow_controls: flow_controls.clone(),
            info: Some(CiphertextFlowControlInfo {
                flow_control_id: flow_control_id.clone(),
                flow_control_policy,
            }),
        });

        self
    }

    /// Mark that this Secure Channel Listener is a Consumer without a known [`FlowControlId`]
    /// It's expected that this Listener is added as a consumer with a known [`FlowControlId`] manually
    /// later. Also, spawned Secure Channels will be marked as Consumers with [`FlowControlId`]
    /// of the message that was used to create the Secure Channel
    pub fn as_consumer(mut self, flow_controls: &FlowControls) -> Self {
        self.consumer_flow_control = Some(CiphertextFlowControl {
            flow_controls: flow_controls.clone(),
            info: None,
        });

        self
    }

    /// Mark spawned Secure Channel Decryptors as Producers for a given Spawner's [`FlowControlId`]
    /// NOTE: Spawned connections get fresh random [`FlowControlId`], however they are still marked
    /// with Spawner's [`FlowControlId`]
    pub fn as_spawner(flow_controls: &FlowControls, flow_control_id: &FlowControlId) -> Self {
        Self {
            consumer_flow_control: None,
            channels_producer_flow_control: Some((flow_controls.clone(), flow_control_id.clone())),
            trust_policy: Arc::new(TrustEveryonePolicy),
            trust_context: None,
            credential: None,
        }
    }

    /// Set trust policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    /// Set credential. Will be presented to every secure channel exchange.
    pub fn with_credential(mut self, credential: Credential) -> Self {
        self.credential = Some(credential);
        self
    }

    /// Set Trust Context.
    pub fn with_trust_context(mut self, trust_context: TrustContext) -> Self {
        self.trust_context = Some(trust_context);
        self
    }

    pub(crate) fn setup_flow_control(
        &self,
        addresses: &Addresses,
        producer_flow_control_id: Option<FlowControlId>,
    ) -> Result<Option<FlowControlId>> {
        match (&self.consumer_flow_control, producer_flow_control_id) {
            (Some(ciphertext_flow_control), Some(producer_flow_control_id)) => {
                // Allow a sender with corresponding flow_control_id send messages to this address
                ciphertext_flow_control.flow_controls.add_consumer(
                    &addresses.decryptor_remote,
                    &producer_flow_control_id,
                    FlowControlPolicy::ProducerAllowMultiple,
                );
            }
            (None, None) => {}
            // We act as a consumer in some cases,
            // but we were reached without a flow_control, which is fine
            (Some(_), None) => {}
            _ => {
                return Err(IdentityError::FlowControlsInconsistency.into());
            }
        }

        match &self.channels_producer_flow_control {
            Some((flow_controls, listener_flow_control_id)) => {
                let flow_control_id = flow_controls.generate_id();
                flow_controls.add_producer(
                    &addresses.decryptor_internal,
                    &flow_control_id,
                    Some(listener_flow_control_id),
                    vec![addresses.encryptor.clone()],
                );

                Ok(Some(flow_control_id))
            }
            None => Ok(None),
        }
    }

    pub(crate) fn create_access_control(
        &self,
        flow_control_id: Option<FlowControlId>,
    ) -> Result<SecureChannelAccessControl> {
        match (&self.channels_producer_flow_control, flow_control_id) {
            (Some((flow_controls, listener_flow_control_id)), Some(flow_control_id)) => {
                let ac = FlowControlOutgoingAccessControl::new(
                    flow_controls.clone(),
                    flow_control_id,
                    Some(listener_flow_control_id.clone()),
                );

                Ok(SecureChannelAccessControl {
                    decryptor_outgoing_access_control: Arc::new(ac),
                })
            }
            (None, None) => Ok(SecureChannelAccessControl {
                decryptor_outgoing_access_control: Arc::new(AllowAll),
            }),
            _ => Err(IdentityError::FlowControlsInconsistency.into()),
        }
    }
}
