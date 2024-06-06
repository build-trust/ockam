use crate::kafka::secure_channel_map::controller::{
    InnerSecureChannelControllerImpl, KafkaSecureChannelControllerImpl,
};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use tokio::sync::MutexGuard;

impl KafkaSecureChannelControllerImpl {
    pub(crate) async fn create_relay(
        inner: &MutexGuard<'_, InnerSecureChannelControllerImpl>,
        context: &Context,
        relay_service: MultiAddr,
        alias: String,
    ) -> ockam_core::Result<()> {
        let relay_info = inner
            .node_manager
            .create_relay(
                context,
                &relay_service,
                alias.clone(),
                None,
                Some(alias.clone()),
            )
            .await?;

        trace!("remote relay created: {relay_info:?}");
        Ok(())
    }
}
