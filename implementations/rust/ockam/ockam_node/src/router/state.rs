//! Router run state utilities
use crate::channel_types::{OneshotReceiver, OneshotSender};
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicBool;
use ockam_core::compat::sync::Mutex as SyncMutex;

// TODO: Merge RouterState and NodeState.
/// Node state
#[derive(Clone)]
pub enum NodeState {
    Running,
    Stopping,
    Stopped,
}

pub struct RouterState {
    node_state: SyncMutex<NodeState>, // TODO: Use AtomicU8 instead and remove is_running field
    termination_senders: SyncMutex<Vec<OneshotSender<()>>>,
    is_running: AtomicBool,
}

impl RouterState {
    pub fn new() -> Self {
        Self {
            node_state: SyncMutex::new(NodeState::Running),
            termination_senders: SyncMutex::new(Default::default()),
            is_running: AtomicBool::new(true),
        }
    }

    /// Set the router state to `Stopping` and return a receiver
    /// to wait for the stop to complete.
    /// When `None` is returned, the router is already terminated.
    pub(super) fn set_to_stopping(&self) -> Option<OneshotReceiver<()>> {
        let mut node_state = self.node_state.lock().unwrap();
        match node_state.deref_mut() {
            NodeState::Running => {
                let (sender, receiver) = crate::channel_types::oneshot_channel();
                *node_state = NodeState::Stopping;
                self.is_running
                    .store(false, core::sync::atomic::Ordering::Relaxed);
                self.termination_senders.lock().unwrap().push(sender);
                Some(receiver)
            }
            NodeState::Stopping => {
                let (sender, receiver) = crate::channel_types::oneshot_channel();
                self.termination_senders.lock().unwrap().push(sender);
                Some(receiver)
            }
            NodeState::Stopped => None,
        }
    }

    /// Set the router to `Stopped` state and notify all tasks waiting for shutdown
    pub(super) fn set_to_stopped(&self) {
        self.is_running
            .store(false, core::sync::atomic::Ordering::Relaxed);
        let previous = {
            let mut guard = self.node_state.lock().unwrap();
            core::mem::replace(guard.deref_mut(), NodeState::Stopped)
        };

        match previous {
            NodeState::Running | NodeState::Stopping => {
                info!("No more workers left. Goodbye!");
                let senders = {
                    let mut guard = self.termination_senders.lock().unwrap();
                    core::mem::take(guard.deref_mut())
                };
                for sender in senders {
                    let _ = sender.send(());
                }
            }
            NodeState::Stopped => {}
        }
    }

    pub(super) async fn wait_until_stopped(&self) {
        let receiver = {
            let guard = self.node_state.lock().unwrap();
            match guard.deref() {
                NodeState::Running | NodeState::Stopping => {
                    let (sender, receiver) = crate::channel_types::oneshot_channel();
                    self.termination_senders.lock().unwrap().push(sender);
                    receiver
                }
                NodeState::Stopped => {
                    return;
                }
            }
        };
        receiver.await.unwrap() // FIXME
    }

    pub(super) fn is_running(&self) -> bool {
        self.is_running.load(core::sync::atomic::Ordering::Relaxed)
    }

    /// Check if this router is still `running`, meaning allows
    /// spawning new workers and processors
    pub(super) fn node_state(&self) -> NodeState {
        self.node_state.lock().unwrap().clone()
    }
}
