use crate::{
    get_secure_channel_participant_id, CredentialProtocolMessage, CredentialSchema, EntityError,
    IssuerWorker, Profile, SecureChannelTrustInfo, TrustPolicy, TrustPolicyImpl,
};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Result, Routed, Worker};
use ockam_node::Context;

pub struct ListenerWorker {
    profile: Profile,
    schema: CredentialSchema,
    trust_policy: TrustPolicyImpl,
}

impl ListenerWorker {
    pub fn new(profile: Profile, schema: CredentialSchema, trust_policy: TrustPolicyImpl) -> Self {
        Self {
            profile,
            schema,
            trust_policy,
        }
    }
}

#[async_trait]
impl Worker for ListenerWorker {
    type Context = Context;
    type Message = CredentialProtocolMessage;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let their_profile_id = get_secure_channel_participant_id(&msg)?;
        let trust_info = SecureChannelTrustInfo::new(their_profile_id.clone());
        let res = self.trust_policy.check(&trust_info)?;

        if !res {
            return Err(EntityError::CredentialTrustCheckFailed.into());
        }

        let return_route = msg.return_route();
        if let CredentialProtocolMessage::IssueOfferRequest(schema_id) = msg.body() {
            if schema_id != self.schema.id {
                return Err(EntityError::SchemaIdDoesntMatch.into());
            }
        } else {
            return Err(EntityError::IssuerListenerInvalidMessage.into());
        }

        let address = Address::random(0);
        let worker = IssuerWorker::new(
            self.profile.clone(),
            their_profile_id,
            self.schema.clone(),
            return_route,
        )?;
        ctx.start_worker(address, worker).await?;

        Ok(())
    }
}
