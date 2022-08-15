//! Pipe behavior modifiers

mod resend;
use dyn_clone::DynClone;
pub use resend::{ReceiverConfirm, SenderConfirm};

mod ordering;
pub use ordering::ReceiverOrdering;

mod handshake;
pub use handshake::HandshakeInit;

use crate::{
    protocols::pipe::{internal::InternalCmd, PipeMessage},
    Context,
};
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{async_trait, Address, Result, Route};

/// Define the behavior of a pipe
#[async_trait]
pub trait BehaviorHook: DynClone + Send {
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
    ) -> Result<PipeModifier>;

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

dyn_clone::clone_trait_object!(BehaviorHook);

/// Indicate to the pipe whether to modify its default behaviour
#[derive(Clone, Copy, Debug)]
pub enum PipeModifier {
    /// No behaviour modification required
    None,
    /// Drop the currently handled message
    Drop,
}

/// Structure to combine a set of pipe BehaviorHooks
pub struct PipeBehavior {
    hooks: Vec<Box<dyn BehaviorHook + Send + Sync + 'static>>,
}

impl Clone for PipeBehavior {
    fn clone(&self) -> Self {
        Self {
            hooks: self
                .hooks
                .iter()
                .map(|b| *dyn_clone::clone_box(b))
                .collect(),
        }
    }
}

impl<T: BehaviorHook + Send + Sync + 'static> From<T> for PipeBehavior {
    fn from(hook: T) -> Self {
        Self::with(hook)
    }
}

impl PipeBehavior {
    pub fn with<T: BehaviorHook + Send + Sync + 'static>(t: T) -> Self {
        Self {
            hooks: vec![Box::new(t)],
        }
    }

    pub fn empty() -> Self {
        Self { hooks: vec![] }
    }

    /// Attach a new BehaviorHook in a chainable manner
    pub fn attach<T: BehaviorHook + Send + Sync + 'static>(mut self, t: T) -> Self {
        self.insert(t);
        self
    }

    /// Insert a new BehaviorHook in place
    pub fn insert<T: BehaviorHook + Send + Sync + 'static>(&mut self, t: T) {
        self.hooks.push(Box::new(t));
    }

    /// Run all external message hooks associated with this pipe
    pub async fn external_all(
        &mut self,
        this: Address,
        peer: Route,
        ctx: &mut Context,
        msg: &PipeMessage,
    ) -> Result<PipeModifier> {
        let mut acc = Vec::with_capacity(self.hooks.len());
        for hook in self.hooks.iter_mut() {
            acc.push(
                hook.on_external(this.clone(), peer.clone(), ctx, msg)
                    .await?,
            );
        }

        // Propagate any drop behaviour
        use PipeModifier as Pm;
        Ok(acc
            .into_iter()
            .fold(Pm::None, |acc, _mod| match (acc, _mod) {
                (Pm::None, Pm::None) => Pm::None,
                (_, Pm::Drop) => Pm::Drop,
                (Pm::Drop, _) => Pm::Drop,
            }))
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
