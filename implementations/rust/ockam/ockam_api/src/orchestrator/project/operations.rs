use crate::nodes::InMemoryNode;
use crate::orchestrator::operation::{Operation, Operations};
use ockam_core::async_trait;
use ockam_node::Context;

#[async_trait]
impl Operations for InMemoryNode {
    #[instrument(skip_all, fields(operation_id = operation_id))]
    async fn get_operation(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<Option<Operation>> {
        self.create_controller()
            .await?
            .get_operation(ctx, operation_id)
            .await
    }

    #[instrument(skip_all, fields(operation_id = operation_id))]
    async fn wait_until_operation_is_complete(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<()> {
        self.create_controller()
            .await?
            .wait_until_operation_is_complete(ctx, operation_id)
            .await
    }
}
