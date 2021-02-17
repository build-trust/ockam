use crate::{error::Error, relay, NodeMessage, NodeReply};
use ockam_core::{Address, Message, Result, Worker};
use std::{future::Future, sync::Arc};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, Sender},
    task::{self, LocalSet},
};

/// Execute a future without blocking the executor
fn block_future<'r, F>(rt: &'r Runtime, f: F) -> <F as Future>::Output
where
    F: Future + Send,
    F::Output: Send,
{
    task::block_in_place(move || {
        let local = LocalSet::new();
        local.block_on(&rt, f)
    })
}

pub struct Context {
    address: Address,
    sender: Sender<NodeMessage>,
    rt: Arc<Runtime>,
}

impl Context {
    pub(crate) fn new(rt: Arc<Runtime>, sender: Sender<NodeMessage>, address: Address) -> Self {
        Self {
            rt,
            sender,
            address,
        }
    }

    pub fn address(&self) -> Address {
        self.address.clone()
    }

    /// Start a new worker handle at [`Address`](ockam_core::Address)
    pub fn start_worker<NM, NW, S>(&self, address: S, worker: NW) -> Result<()>
    where
        S: Into<Address>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        let tx = self.sender.clone();
        let rt = self.rt.clone();
        let address = address.into();

        // Wait for a worker to have started to avoid data races when
        // sending messages to it in subsequent api calls
        block_future(&self.rt, async move {
            let ctx = Context::new(rt.clone(), tx.clone(), address.clone());
            let sender = relay::build::<NW, NM>(rt.as_ref(), worker, ctx);

            let msg = NodeMessage::start_worker(address, sender);
            let _result: Result<()> = match tx.send(msg).await {
                Ok(()) => Ok(()),
                Err(_e) => Err(Error::FailedStartWorker.into()),
            };
        });

        Ok(())
    }

    /// Signal to the local application runner to shut down
    pub fn stop(&self) -> Result<()> {
        let tx = self.sender.clone();
        tokio::spawn(async move {
            println!("App: shutting down all workers");
            let _result: Result<()> = match tx.send(NodeMessage::StopNode).await {
                Ok(()) => Ok(()),
                Err(_e) => Err(Error::FailedStopNode.into()),
            };
        });

        Ok(())
    }

    /// Send a message to a particular worker
    pub fn send_message<S, M>(&self, address: S, msg: M) -> Result<()>
    where
        S: Into<Address>,
        M: Message + Send + 'static,
    {
        let address = address.into();
        let tx = self.sender.clone();
        tokio::spawn(async move {
            let (reply_tx, mut reply_rx) = channel(1);
            let req = NodeMessage::SenderReq(address, reply_tx);

            // FIXME/ DESIGN: error communication concept
            let _result: Result<()> = match tx.send(req).await {
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

    /// Return a list of all available worker addresses on a node
    pub fn list_workers(&self) -> Result<Vec<Address>> {
        let tx = self.sender.clone();

        block_future(&self.rt, async move {
            let (msg, mut reply_rx) = NodeMessage::list_workers();

            match tx.send(msg).await {
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
}
