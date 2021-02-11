use crate::{error::Error, relay, Context, NodeMessage, NodeReply};
use ockam_core::{Address, Message, Result, Worker};

use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Sender};

#[derive(Clone)]
pub struct Node {
    sender: Sender<NodeMessage>,
    rt: Arc<Runtime>,
}

impl Node {
    pub fn new(sender: Sender<NodeMessage>, rt: Arc<Runtime>) -> Self {
        Self { sender, rt }
    }

    /// Shut down a worker with a specific address
    pub async fn stop_worker<S: ToString>(&self, address: S) -> Result<()> {
        let (reply_tx, mut reply_rx) = channel(1);
        let msg = NodeMessage::StopWorker(address.to_string(), reply_tx);

        match self.sender.send(msg).await {
            Ok(()) => {
                if let Some(NodeReply::Ok) = reply_rx.recv().await {
                    Ok(())
                } else {
                    Err(Error::FailedStopWorker.into())
                }
            }
            Err(_e) => Err(Error::FailedStopWorker.into()),
        }
    }

    /// Send messages to all workers to shut them down
    pub async fn stop(&self) -> Result<()> {
        println!("Node: shutting down all workers");

        match self.sender.send(NodeMessage::StopNode).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStopNode.into()),
        }
    }

    /// Create and start the handler at [`Address`](ockam_core::Address).
    pub async fn start_worker<S, W, M>(&self, address: S, worker: W) -> Result<()>
    where
        S: ToString,
        W: Worker<Context = Context, Message = M>,
        M: Message + Send + 'static,
    {
        let address = address.to_string();
        let ctx = Context::new(self.clone(), address.clone());
        let sender = relay::build(self.rt.as_ref(), worker, ctx);

        let msg = NodeMessage::start_worker(address, sender);
        match self.sender.send(msg).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStartWorker.into()),
        }
    }

    /// Return a list of all available worker addresses on a node
    pub async fn list_workers(&self) -> Result<Vec<Address>> {
        let (msg, mut reply_rx) = NodeMessage::list_workers();

        match self.sender.send(msg).await {
            Ok(()) => {
                if let Some(NodeReply::Workers(list)) = reply_rx.recv().await {
                    Ok(list)
                } else {
                    Ok(vec![])
                }
            }
            Err(_e) => Err(Error::FailedListWorker.into()),
        }
    }

    /// Send a message to a particular worker
    pub async fn send_message<S, M>(&self, address: S, msg: M) -> Result<()>
    where
        S: ToString,
        M: Message + Send + 'static,
    {
        let address = address.to_string();
        let (reply_tx, mut reply_rx) = channel(1);
        let req = NodeMessage::SenderReq(address, reply_tx);

        match self.sender.send(req).await {
            Ok(()) => {
                if let Some(NodeReply::Sender(_, s)) = reply_rx.recv().await {
                    let msg = msg.encode()?;
                    match s.send(msg).await {
                        Ok(()) => Ok(()),
                        Err(_e) => Err(Error::FailedSendMessage.into()),
                    }
                } else {
                    Err(Error::FailedSendMessage.into())
                }
            }
            Err(_e) => Err(Error::FailedSendMessage.into()),
        }
    }

    /// Block to receive a message for the current worker
    pub async fn receive<M>(&self, ctx: &Context) -> Result<M>
    where
        M: Message + Send + 'static,
    {
        let _addr = ctx.address();
        todo!()
    }
}
