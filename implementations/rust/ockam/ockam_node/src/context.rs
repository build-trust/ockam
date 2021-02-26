use crate::{
    error::Error,
    relay::{self, RelayMessage},
    Cancel, Mailbox, NodeMessage, NodeReply,
};
use ockam_core::{Address, AddressSet, Message, Result, Worker};
use std::sync::Arc;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, Sender},
};

pub struct Context {
    address: AddressSet,
    msg_addr: Option<Address>,
    sender: Sender<NodeMessage>,
    rt: Arc<Runtime>,
    pub(crate) mailbox: Mailbox,
}

impl Context {
    pub(crate) fn new(
        rt: Arc<Runtime>,
        sender: Sender<NodeMessage>,
        address: AddressSet,
        mailbox: Mailbox,
    ) -> Self {
        Self {
            rt,
            sender,
            address,
            msg_addr: None,
            mailbox,
        }
    }

    /// Override the worker address for a specific message address
    pub(crate) fn message_address<A: Into<Option<Address>>>(&mut self, a: A) {
        self.msg_addr = a.into();
    }

    /// Return the current context address
    ///
    /// During initialisation and shutdown this will return the
    /// primary worker address.  During message handling it will
    /// return the address the message was addressed to.
    pub fn address(&self) -> Address {
        self.msg_addr
            .clone()
            .or(Some(self.address.first().clone()))
            .unwrap()
    }

    /// Start a new worker handle at [`Address`](ockam_core::Address)
    pub async fn start_worker<NM, NW, S>(&self, address: S, worker: NW) -> Result<()>
    where
        S: Into<AddressSet>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        let address = address.into();

        // Build the mailbox first
        let (mb_tx, mb_rx) = channel(32);
        let mb = Mailbox::new(mb_rx, mb_tx.clone());

        // Pass it to the context
        let ctx = Context::new(self.rt.clone(), self.sender.clone(), address.clone(), mb);

        // Then initialise the worker message relay
        let sender = relay::build::<NW, NM>(self.rt.as_ref(), worker, ctx);

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

    /// Send a message to a particular worker
    pub async fn send_message<S, M>(&self, address: S, msg: M) -> Result<()>
    where
        S: Into<Address>,
        M: Message + Send + 'static,
    {
        let address = address.into();
        let (reply_tx, mut reply_rx) = channel(1);
        let req = NodeMessage::SenderReq(address.clone(), reply_tx);

        match self.sender.send(req).await {
            Ok(()) => {
                if let Some(NodeReply::Sender(_, s)) = reply_rx.recv().await {
                    let msg = msg.encode().unwrap();
                    match s.send(RelayMessage::new(address, msg)).await {
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

    /// Block the current worker to wait for a typed message
    ///
    /// Will return `None` if the corresponding worker has been
    /// stopped, or the underlying Node has shut down.
    pub async fn receive<'ctx, M: Message>(&'ctx mut self) -> Result<Cancel<'ctx, M>> {
        self.mailbox
            .next()
            .await
            .and_then(|RelayMessage { addr, data }| {
                M::decode(&data).ok().map(move |msg| (msg, addr))
            })
            .map(move |(msg, addr)| Cancel::new(msg, addr, self))
            .ok_or_else(|| Error::FailedLoadData.into())
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
