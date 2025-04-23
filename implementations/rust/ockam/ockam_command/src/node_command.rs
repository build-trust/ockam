use async_trait::async_trait;
use ockam_api::nodes::{InMemoryNode, InMemoryNodeBuilder};
use ockam_api::CliState;

use ockam_api::cli_state::NamedIdentity;
use ockam_api::orchestrator::project::Project;
use ockam_api::orchestrator::AuthorityNodeClient;
use ockam_node::Context;
use std::sync::Arc;
use std::time::Duration;

/// This supports the instantiation and shutdown of an in-memory node
/// which can be used to execute a command.
///
/// Some optional values can be provided:
///
///  - A project name (used to access the controller for example)
///  - An identity name (used to create secure channels)
///  - A default timeout for creating secure channels
///
/// The `run` must be implemented by the command that uses this trait.
/// It is also possible to provide an init method that will be called before the run method,
/// for example to create the identity which will be used to create the in-memory node.
///
#[async_trait]
pub trait InMemoryNodeCommand<T: Send + Sync + 'static = ()>:
    Clone + Send + Sync + 'static
{
    /// Optional project name
    fn project_name(&self) -> Option<String> {
        None
    }

    /// Optional identity name
    fn identity_name(&self) -> Option<String> {
        None
    }

    /// Optional default timeout for creating secure channels
    fn timeout(&self) -> Option<Duration> {
        None
    }

    /// This method is called before the run method.
    async fn init(&self) -> miette::Result<()> {
        Ok(())
    }

    /// This method needs to be implemented. It guarantees that the in-memory node is properly shutdown
    /// when the command is finished or raises an error.
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<T>;

    /// The default execute method ensures that the command code will be executed with a node that is properly shutdown
    async fn execute(&self, ctx: &Context, state: Arc<CliState>) -> miette::Result<T> {
        self.init().await?;
        let self_clone = Arc::new(self.clone());
        InMemoryNodeBuilder::create(ctx, state)?
            .with_identity_name(self.identity_name())
            .with_project_name(self.project_name())
            .with_timeout(self.timeout())
            .run({
                let self_clone = self_clone.clone();
                move |n| {
                    let self_clone = self_clone.clone();
                    Box::pin(async move { self_clone.run(n).await })
                }
            })
            .await
    }

    /// Return an authority client for the given project and identity
    async fn authority_client(
        &self,
        node: Arc<InMemoryNode>,
    ) -> miette::Result<AuthorityNodeClient> {
        let identity = node
            .state()
            .get_identity_name_or_default(&self.identity_name())
            .await?;

        let project = node
            .state()
            .projects()
            .get_project_by_name_or_default(&self.project_name())
            .await?;
        node.create_authority_client_with_project(&project, Some(identity), false)
            .await
    }

    /// Return the project specified by this command
    async fn get_project(&self, node: Arc<InMemoryNode>) -> miette::Result<Project> {
        Ok(node
            .state()
            .projects()
            .get_project_by_name_or_default(&self.project_name())
            .await?)
    }

    /// Return the identity specified by this command
    async fn get_identity(&self, node: Arc<InMemoryNode>) -> miette::Result<NamedIdentity> {
        Ok(node
            .state()
            .get_named_identity_or_default(&self.identity_name())
            .await?)
    }
}
