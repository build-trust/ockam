use crate::{error::Error, relay, Context, NodeMessage, NodeReply};
use ockam_core::{Address, Message, Result, Worker};

use std::{future::Future, sync::Arc};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, Sender},
    task::{self, LocalSet},
};

#[derive(Clone)]
pub struct Node {
    sender: Sender<NodeMessage>,
    rt: Arc<Runtime>,
}

/// Execute a future without blocking the executor
fn block_future<'r, F>(rt: &'r Runtime, f: F) -> <F as Future>::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    task::block_in_place(move || {
        let local = LocalSet::new();
        local.block_on(&rt, f)
    })
}

impl Node {
    pub fn new(sender: Sender<NodeMessage>, rt: Arc<Runtime>) -> Self {
        Self { sender, rt }
    }

    /// Shut down a worker with a specific address
    pub fn stop_worker<S: Into<Address>>(self: &Arc<Self>, address: S) -> Result<()> {
        let this = Arc::clone(self);
        let address = address.into();

        tokio::spawn(async move {
            let (reply_tx, mut reply_rx) = channel(1);
            let msg = NodeMessage::StopWorker(address, reply_tx);

            let _result: Result<()> = match this.sender.send(msg).await {
                Ok(()) => {
                    if let Some(NodeReply::Ok) = reply_rx.recv().await {
                        Ok(())
                    } else {
                        Err(Error::FailedStopWorker.into())
                    }
                }
                Err(_e) => Err(Error::FailedStopWorker.into()),
            };
        });

        Ok(())
    }

    /// Send messages to all workers to shut them down
    pub fn stop(&self) -> Result<()> {
        println!("Node: shutting down all workers");

        let this = self.clone();
        tokio::spawn(async move {
            let _result: Result<()> = match this.sender.send(NodeMessage::StopNode).await {
                Ok(()) => Ok(()),
                Err(_e) => Err(Error::FailedStopNode.into()),
            };
        });

        Ok(())
    }

    /// Create and start the handler at [`Address`](ockam_core::Address).
    pub fn start_worker<S, W, M>(&self, address: S, worker: W) -> Result<()>
    where
        S: Into<Address>,
        W: Worker<Context = Context, Message = M>,
        M: Message + Send + 'static,
    {
        let this = self.clone();
        let address = address.into();

        // Wait for a worker to have started to avoid data races when
        // sending messages to it in subsequent api calls
        block_future(&self.rt, async move {
            let ctx = Context::new(this.clone(), address.clone());
            let sender = relay::build(this.rt.as_ref(), worker, ctx);

            let msg = NodeMessage::start_worker(address, sender);
            let _result: Result<()> = match this.sender.send(msg).await {
                Ok(()) => Ok(()),
                Err(_e) => Err(Error::FailedStartWorker.into()),
            };
        });

        Ok(())
    }

    /// Return a list of all available worker addresses on a node
    pub fn list_workers(&self) -> Result<Vec<Address>> {
        let this = self.clone();

        block_future(&self.rt, async move {
            let (msg, mut reply_rx) = NodeMessage::list_workers();

            match this.sender.send(msg).await {
                Ok(()) => {
                    if let Some(NodeReply::Workers(list)) = reply_rx.recv().await {
                        Ok(list)
                    } else {
                        Ok(vec![])
                    }
                }
                Err(_e) => Err(Error::FailedListWorker.into()),
            }
        })
    }

    /// Send a message to a particular worker
    pub fn send_message<S, M>(&self, address: S, msg: M) -> Result<()>
    where
        S: Into<Address>,
        M: Message + Send + 'static,
    {
        let address = address.into();
        let this = self.clone();
        tokio::spawn(async move {
            let (reply_tx, mut reply_rx) = channel(1);
            let req = NodeMessage::SenderReq(address, reply_tx);

            // FIXME/ DESIGN: error communication concept
            let _result: Result<()> = match this.sender.send(req).await {
                Ok(()) => {
                    if let Some(NodeReply::Sender(_, s)) = reply_rx.recv().await {
                        let msg = msg.encode().unwrap();
                        match s.send(msg).await {
                            Ok(()) => Ok(()),
                            Err(_e) => Err(Error::FailedSendMessage.into()),
                        }
                    } else {
                        Err(Error::FailedSendMessage.into())
                    }
                }
                Err(_e) => Err(Error::FailedSendMessage.into()),
            };
        });

        Ok(())
    }

    /// Block to receive a message for the current worker
    pub fn receive<M>(&self, ctx: &Context) -> Result<M>
    where
        M: Message + Send + 'static,
    {
        self.rt.block_on(async move {
            let _addr = ctx.address();
            todo!()
        })
    }
}
