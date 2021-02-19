use crate::{error::Error, relay, Cancel, Mailbox, NodeMessage, NodeReply};
use ockam_core::{Address, Message, Result, Worker};
use std::sync::Arc;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, Sender},
};

pub struct Context {
    address: Address,
    sender: Sender<NodeMessage>,
    rt: Arc<Runtime>,
    pub(crate) mailbox: Mailbox,
}

impl ContextTrait for Context {
    fn propagate_failure(&self, _e: ockam_core::Error) {}
}

impl Context {
    pub(crate) fn new(
        rt: Arc<Runtime>,
        sender: Sender<NodeMessage>,
        address: Address,
        mailbox: Mailbox,
    ) -> Self {
        Self {
            rt,
            sender,
            address,
            mailbox,
        }
    }

    pub fn address(&self) -> Address {
        self.address.clone()
    }

    /// Start a new worker and become its supervisor
    ///
    /// Calls `start_worker` under the hood, while also setting up the
    /// worker with additional back-channel metadata.
    pub fn supervise<NM, NW, S>(&self, address: S, worker: NW) -> Result<()>
    where
        S: Into<Address>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        self.create_worker(address, worker, Some(self.address()))
    }

    /// Start a new worker handle at [`Address`](ockam_core::Address)
    pub async fn start_worker<NM, NW, S>(&self, address: S, worker: NW) -> Result<()>
    where
        S: Into<Address>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
<<<<<<< HEAD
=======
        self.create_worker(address, worker, None)
    }

    fn create_worker<NM, NW, S>(
        &self,
        address: S,
        worker: NW,
        supervisor: impl Into<Option<Address>>,
    ) -> Result<()>
    where
        S: Into<Address>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        let tx = self.sender.clone();
        let rt = self.rt.clone();
>>>>>>> feat(rust): add worker supervisor concept with error backchannel
        let address = address.into();
        let supervisor = supervisor.into();

        // Build the mailbox first
        let (mb_tx, mb_rx) = channel(32);
        let mb = Mailbox::new(mb_rx, mb_tx.clone());

        // Pass it to the context
        let ctx = Context::new(self.rt.clone(), self.sender.clone(), address.clone(), mb);

<<<<<<< HEAD
        // Then initialise the worker message relay
        let sender = relay::build::<NW, NM>(self.rt.as_ref(), worker, ctx);
=======
            // Then initialise the worker message relay
            let sender = relay::build::<NW, NM>(rt.as_ref(), worker, ctx, supervisor);
>>>>>>> feat(rust): add worker supervisor concept with error backchannel

        let msg = NodeMessage::start_worker(address, sender);
        let _result: Result<()> = match self.sender.send(msg).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStartWorker.into()),
        };

        Ok(())
    }

    /// Signal to the local application runner to shut down
    pub async fn stop(&self) -> Result<()> {
        let tx = self.sender.clone();
        println!("App: shutting down all workers");
        let _result: Result<()> = match tx.send(NodeMessage::StopNode).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStopNode.into()),
        };

        Ok(())
    }

    pub fn stop_worker<S: Into<Address>>(&self, address: S) -> Result<()> {
        let address = address.into();
        let tx = self.sender.clone();
        block_future(&self.rt, async move {
            let (reply_tx, mut reply_rx) = channel(1);

            match tx.send(NodeMessage::StopWorker(address, reply_tx)).await {
                Ok(()) => match reply_rx.recv().await.unwrap() {
                    NodeReply::Ok => Ok(()),
                    _ => Err(Error::FailedStopWorker.into()),
                },
                Err(_e) => Err(Error::FailedStopWorker.into()),
            }
        })
    }

    /// Send a message to a particular worker
    pub async fn send_message<S, M>(&self, address: S, msg: M) -> Result<()>
    where
        S: Into<Address>,
        M: Message + Send + 'static,
    {
        let address = address.into();
        let (reply_tx, mut reply_rx) = channel(1);
        let req = NodeMessage::SenderReq(address, reply_tx);

        // FIXME/ DESIGN: error communication concept
        let _result: Result<()> = match self.sender.send(req).await {
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

        Ok(())
    }

    /// Block the current worker to wait for a typed message
    ///
    /// Will return `None` if the corresponding worker has been
    /// stopped, or the underlying Node has shut down.
    pub async fn receive<'ctx, M: Message>(&'ctx mut self) -> Option<Cancel<'ctx, M>> {
        self.mailbox
            .next()
            .await
            .and_then(|enc| M::decode(&enc).ok())
            .map(move |msg| Cancel::new(msg, self))
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
}
