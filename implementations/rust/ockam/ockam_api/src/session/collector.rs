use crate::session::ping::Ping;
use ockam::Worker;
use ockam_core::{Error, Routed};
use ockam_node::tokio::sync::mpsc;
use ockam_node::Context;

/// A collector receives echo messages and forwards them.
#[derive(Debug)]
pub(super) struct Collector(mpsc::Sender<Ping>);

impl Collector {
    pub fn new(sender: mpsc::Sender<Ping>) -> Self {
        Self(sender)
    }
}

#[ockam::worker]
impl Worker for Collector {
    type Message = Ping;
    type Context = Context;

    async fn handle_message(
        &mut self,
        _: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<(), Error> {
        if self.0.send(msg.into_body()?).await.is_err() {
            debug!("collector could not send message to session")
        }
        Ok(())
    }
}
