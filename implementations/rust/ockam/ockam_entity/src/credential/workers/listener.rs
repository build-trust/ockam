use crate::{
    get_secure_channel_participant_id, CredentialProtocolMessage, CredentialSchema, EntityError,
    IssuerWorker, Profile, SecureChannelTrustInfo, TrustPolicy, TrustPolicyImpl,
};
use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, AsyncTryClone, NodeContext};
use ockam_core::{Address, Result, Routed, Worker};

pub struct ListenerWorker<C> {
    profile: Profile<C>,
    schema: CredentialSchema,
    trust_policy: TrustPolicyImpl<C>,
}

impl<C: NodeContext> ListenerWorker<C> {
    pub fn new(
        profile: Profile<C>,
        schema: CredentialSchema,
        trust_policy: TrustPolicyImpl<C>,
    ) -> Self {
        Self {
            profile,
            schema,
            trust_policy,
        }
    }
}

#[async_trait]
impl<C: NodeContext> Worker<C> for ListenerWorker<C> {
    type Message = CredentialProtocolMessage;

    async fn handle_message(&mut self, ctx: &mut C, msg: Routed<Self::Message>) -> Result<()> {
        let their_profile_id = get_secure_channel_participant_id(&msg)?;
        let trust_info = SecureChannelTrustInfo::new(their_profile_id.clone());
        let res = self.trust_policy.check(&trust_info).await?;

        if !res {
            return Err(EntityError::CredentialTrustCheckFailed.into());
        }

        let return_route = msg.return_route();
        if let CredentialProtocolMessage::IssueOfferRequest(schema_id) = msg.body() {
            if schema_id != self.schema.id {
                return Err(EntityError::SchemaIdDoesNotMatch.into());
            }
        } else {
            return Err(EntityError::IssuerListenerInvalidMessage.into());
        }

        let address = Address::random(0);
        let worker = IssuerWorker::new(
            self.profile.async_try_clone().await?,
            their_profile_id,
            self.schema.clone(),
            return_route,
        )?;
        ctx.start_worker(address.into(), worker).await?;

        Ok(())
    }
}
