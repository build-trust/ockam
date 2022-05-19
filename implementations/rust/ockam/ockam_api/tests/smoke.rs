use ockam_api::nodes;
use ockam_core::Result;
use ockam_node::Context;

#[ockam_macros::test]
async fn smoke(ctx: &mut Context) -> Result<()> {
    ctx.start_worker("nodes", nodes::Server::default()).await?;

    let mut client = nodes::Client::new("nodes".into(), ctx).await?;

    // create a node
    let a = client
        .create_node(&nodes::types::CreateNode::new("first"))
        .await?;
    let i = a.id().to_string();

    // get the node info for the identifier received
    let b = client.get(&i).await?;
    assert_eq!(i, b.id());

    let c = client.list().await?;
    assert_eq!(1, c.len());

    ctx.stop().await?;
    Ok(())
}
