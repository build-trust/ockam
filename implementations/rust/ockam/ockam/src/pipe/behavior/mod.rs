//! Pipe behavior modifiers

mod resend;
pub use resend::{ReceiverConfirm, SenderConfirm};

use crate::protocols::pipe::{internal::InternalCmd, PipeMessage};
use ockam_core::{async_trait, Address, Result, Route};
use ockam_node::Context;

/// Define the behavior of a pipe
#[async_trait]
pub trait BehaviorHook {
    /// This function MUST be run for every incoming user message
    ///
    /// * Access to mutable self
    /// * Access to own internal address
    /// * Access to peer internal route
    /// * Access to mutable context
    /// * Access to incoming or outgoing PipeMessage
    async fn on_external(
        &mut self,
        this: Address,
        peer: Route,
        ctx: &mut Context,
        msg: &PipeMessage,
    ) -> Result<()>;

    /// This function MUST be run for every incoming internal message
    ///
    /// An internal message is one sent to the private API address of
    /// this worker to issue commands, such as re-sending payloads
    async fn on_internal(
        &mut self,
        this: Address,
        peer: Route,
        ctx: &mut Context,
        msg: &InternalCmd,
    ) -> Result<()>;
}

/// Structure to combine a set of pipe BehaviorHooks
pub struct PipeBehavior {
    hooks: Vec<Box<dyn BehaviorHook + Send + 'static>>,
}

impl PipeBehavior {
    pub fn with<T: BehaviorHook + Send + 'static>(t: T) -> Self {
        Self {
            hooks: vec![Box::new(t)],
        }
    }

    pub fn empty() -> Self {
        Self { hooks: vec![] }
    }

    pub fn add<T: BehaviorHook + Send + 'static>(mut self, t: T) -> Self {
        self.hooks.push(Box::new(t));
        self
    }

    /// Run all external message hooks associated with this pipe
    pub async fn external_all(
        &mut self,
        this: Address,
        peer: Route,
        ctx: &mut Context,
        msg: &PipeMessage,
    ) -> Result<()> {
        for hook in self.hooks.iter_mut() {
            hook.on_external(this.clone(), peer.clone(), ctx, msg)
                .await?;
        }

        Ok(())
    }

    /// Run all internal message hooks associated with this pipe
    pub async fn internal_all(
        &mut self,
        this: Address,
        peer: Route,
        ctx: &mut Context,
        msg: &InternalCmd,
    ) -> Result<()> {
        for hook in self.hooks.iter_mut() {
            hook.on_internal(this.clone(), peer.clone(), ctx, msg)
                .await?;
        }

        Ok(())
    }
}
