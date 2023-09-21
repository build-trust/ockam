use crate::Context;
use crate::{error::*, NodeMessage, ShutdownType};
use ockam_core::{
    errcode::{Kind, Origin},
    Error, Result,
};

impl Context {
    /// Signal to the local runtime to shut down immediately
    ///
    /// **WARNING**: calling this function may result in data loss.
    /// It is recommended to use the much safer
    /// [`Context::stop`](Context::stop) function instead!
    pub async fn stop_now(&self) -> Result<()> {
        let tx = self.sender.clone();
        info!("Immediately shutting down all workers");
        let (msg, _) = NodeMessage::stop_node(ShutdownType::Immediate);

        match tx.send(msg).await {
            Ok(()) => Ok(()),
            Err(e) => Err(Error::new(Origin::Node, Kind::Invalid, e)),
        }
    }

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed.
    /// The default timeout for a safe shutdown is 1 second.  You can
    /// change this behaviour by calling
    /// [`Context::stop_timeout`](Context::stop_timeout) directly.
    pub async fn stop(&self) -> Result<()> {
        self.stop_timeout(1).await
    }

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed
    /// or the desired timeout has been reached.
    pub async fn stop_timeout(&self, seconds: u8) -> Result<()> {
        let (req, mut rx) = NodeMessage::stop_node(ShutdownType::Graceful(seconds));
        self.sender
            .send(req)
            .await
            .map_err(NodeError::from_send_err)?;

        // Wait until we get the all-clear
        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;
        Ok(())
    }
}
