use crate::kafka::secure_channel_map::controller::{
    InnerSecureChannelControllerImpl, KafkaSecureChannelControllerImpl,
};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::Context;
use tokio::sync::MutexGuard;

impl KafkaSecureChannelControllerImpl {
    pub(crate) async fn create_relay(
        inner: &MutexGuard<'_, InnerSecureChannelControllerImpl>,
        context: &Context,
        relay_service: MultiAddr,
        alias: String,
    ) -> ockam_core::Result<()> {
        let is_rust_node = {
            // we might create a relay in the producer passing through a project relay
            !(relay_service.starts_with(Project::CODE)
                && relay_service
                    .last()
                    .map_or(false, |p| p.code() == Project::CODE))
        };

        let relay_info = inner
            .node_manager
            .create_relay(
                context,
                &relay_service,
                alias.clone(),
                is_rust_node,
                None,
                Some(alias.clone()),
            )
            .await?;

        trace!("remote relay created: {relay_info:?}");
        Ok(())
    }
}
