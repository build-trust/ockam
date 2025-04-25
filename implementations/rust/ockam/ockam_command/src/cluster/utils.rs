use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_node::Context;

pub async fn get_api_client(
    node: &InMemoryNode,
    use_http_api: bool,
) -> miette::Result<Box<dyn AiPlatformApi + Send + Sync + 'static>> {
    if use_http_api {
        Ok(Box::new(node.clone()))
    } else {
        Ok(Box::new(node.create_controller().await?))
    }
}

pub async fn get_cluster(ctx: &Context, node: &InMemoryNode) -> miette::Result<String> {
    let controller_client = node.create_controller().await?;
    Ok(controller_client.get_cluster(ctx).await?.into_inner())
}
