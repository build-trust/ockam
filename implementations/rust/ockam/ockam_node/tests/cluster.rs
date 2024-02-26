use ockam_core::{async_trait, Processor, Result};
use ockam_node::{Context, NodeBuilder};

struct ClusterProcessor;

#[async_trait]
impl Processor for ClusterProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster("TEST_CLUSTER").await?;

        Ok(())
    }
}

#[allow(non_snake_case)]
#[test]
fn stop_node__cluster_processor__should_not_fail() {
    for _ in 0..100 {
        let (ctx, mut executor) = NodeBuilder::new().build();
        executor
            .execute(async move {
                ctx.start_processor("test", ClusterProcessor).await?;

                ctx.stop().await?;

                Ok(())
            })
            .unwrap()
            .unwrap()
    }
}
