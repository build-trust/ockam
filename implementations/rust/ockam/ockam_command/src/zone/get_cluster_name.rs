use crate::cluster::utils::get_cluster;
use crate::node_command::InMemoryNodeCommand;
use async_trait::async_trait;
use ockam_api::nodes::InMemoryNode;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct GetClusterName;

#[async_trait]
impl InMemoryNodeCommand<String> for GetClusterName {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<String> {
        let ctx = node.ctx();
        get_cluster(ctx, &node).await
    }
}
