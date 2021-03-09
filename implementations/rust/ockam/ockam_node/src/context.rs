use crate::{
    error::Error,
    relay::{self, RelayMessage},
    Cancel, Mailbox, NodeMessage,
};
use ockam_core::{Address, AddressSet, Message, Result, Route, TransportMessage, Worker};
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

    /// Send a message via a fully qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockem_core::RouteBuilder
    pub async fn send_message<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        let route = route.into();
        let (reply_tx, mut reply_rx) = channel(1);
        let req = NodeMessage::SenderReq(route.clone(), reply_tx);

        // First resolve the next hop in the route
        self.sender.send(req).await.map_err(|e| Error::from(e))?;
        let (addr, sender, needs_wrapping) = reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_sender()?;

        // Pack the payload into a TransportMessage
        let payload = msg.encode().unwrap();
        let data = TransportMessage::v1(route.clone(), payload);

        // Pack transport message into relay message wrapper
        let msg = if needs_wrapping {
            RelayMessage::pre_router(addr, data)
        } else {
            RelayMessage::direct(addr, data)
        };

        // Send the packed user message with associated route
        sender.send(msg).await.map_err(|e| Error::from(e))?;

        Ok(())
    }

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`Context::send_message`], but taking a
    /// [`TransportMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`Context::send_message`] instead, unless you are writing an
    /// external router implementation for ockam node.
    ///
    /// [`Context::send_message`]: crate::Context::send_message
    /// [`TransportMessage`]: ockam_core::TransportMessage
    pub async fn forward_message(&self, data: TransportMessage) -> Result<()> {
        // Resolve the sender for the next hop in the messages route
        let route = data.onward.clone();
        let (reply_tx, mut reply_rx) = channel(1);
        let req = NodeMessage::SenderReq(route, reply_tx);

        // First resolve the next hop in the route
        self.sender.send(req).await.map_err(|e| Error::from(e))?;
        let (addr, sender, _) = reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_sender()?;

        // Pack the transport message into a relay message
        let msg = RelayMessage::direct(addr, data);
        sender.send(msg).await.map_err(|e| Error::from(e))?;

        Ok(())
    }

    /// Block the current worker to wait for a typed message
    ///
    /// Will return `None` if the corresponding worker has been
    /// stopped, or the underlying Node has shut down.
    pub async fn receive<'ctx, M: Message>(&'ctx mut self) -> Result<Cancel<'ctx, M>> {
        self.mailbox
            .next()
            .await
            .and_then(|relay_msg| {
                let (addr, data) = relay_msg.transport();

                M::decode(&data.payload)
                    .ok()
                    .map(move |msg| (msg, data, addr))
            })
            .map(move |(msg, data, addr)| Cancel::new(msg, data, addr, self))
            .ok_or_else(|| Error::FailedLoadData.into())
    }

    /// Return a list of all available worker addresses on a node
    pub async fn list_workers(&self) -> Result<Vec<Address>> {
        let (msg, mut reply_rx) = NodeMessage::list_workers();

        self.sender.send(msg).await.map_err(|e| Error::from(e))?;

        Ok(reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_workers()?)
    }

    /// Register a router for a specific address type
    pub async fn register<A: Into<Address>>(&self, type_: u8, addr: A) -> Result<()> {
        let addr = addr.into();
        let (tx, mut rx) = channel(1);
        self.sender
            .send(NodeMessage::Router(type_, addr, tx))
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        Ok(rx.recv().await.ok_or(Error::InternalIOFailure)??.is_ok()?)
    }
}
