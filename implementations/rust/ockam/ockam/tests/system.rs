use ockam::{Context, OckamMessage, SystemHandler, WorkerSystem};
use ockam_core::compat::{collections::BTreeMap, string::String};
use ockam_core::{
    Address, AllowAll, Any, Decodable, LocalMessage, Mailbox, Mailboxes, Message, Result, Routed,
    Worker,
};
use ockam_node::WorkerBuilder;
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
struct TestWorker {
    system: WorkerSystem<Context, Any>,
}

/// A very simple System Handler which takes incoming messages and
/// forwards them to the next handler in their chain.
#[derive(Clone)]
struct StepHandler {
    next: Address,
}

impl StepHandler {
    fn new<A: Into<Address>>(next: A) -> Self {
        Self { next: next.into() }
    }
}

#[ockam_core::async_trait]
impl<M: Message> SystemHandler<Context, M> for StepHandler {
    // We implement this function only to make rustc happy.  We have
    // already been initialised fully in this test so this is not
    // required.
    async fn initialize(
        &mut self,
        _: &mut Context,
        _: &mut BTreeMap<String, Address>,
    ) -> Result<()> {
        Ok(())
    }

    async fn handle_message(
        &mut self,
        _self_addr: Address,
        ctx: &mut Context,
        msg: Routed<M>,
    ) -> Result<()> {
        info!("Handling message via StepHandler");
        let mut msg = msg.into_transport_message();
        msg.onward_route
            .modify()
            .pop_front()
            .prepend(self.next.clone());
        ctx.forward(LocalMessage::new(msg, vec![])).await
    }
}

#[ockam::worker]
impl Worker for TestWorker {
    type Context = Context;
    type Message = Any;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.system.handle_message(ctx, msg).await
    }
}

#[ockam::test]
async fn send_messages(ctx: &mut Context) -> Result<()> {
    // Initialise the TestWorker system
    let mut w = TestWorker::default();

    // Each handler is given an address to forward messages to.  But
    // this is _very_ dependente on the type of handler that is being
    // initialised.  Also: this system MUST interact with the
    // MetadataMessage, meaning that for some System Handlers it is
    // possible to get the "next" address from the metadata section.
    w.system.attach("worker.1", StepHandler::new("worker.2"));
    w.system.attach("worker.2", StepHandler::new("child"));

    // FIXME: @ac
    let mailboxes = Mailboxes::new(
        Mailbox::new("worker", Arc::new(AllowAll), Arc::new(AllowAll)),
        vec![
            Mailbox::new("worker.1", Arc::new(AllowAll), Arc::new(AllowAll)),
            Mailbox::new("worker.2", Arc::new(AllowAll), Arc::new(AllowAll)),
        ],
    );

    // Start the worker with three publicly mapped addresses
    WorkerBuilder::with_mailboxes(mailboxes, w)
        .start(ctx)
        .await?;

    // Send a message and wait for a reply
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send("worker.1", String::from("Hello Ockam!"))
        .await?;
    let msg = child_ctx.receive::<String>().await?;
    info!("Received message '{}'", msg);

    // Shut down the test
    ctx.stop().await
}

#[derive(Clone)]
struct AddMetadata {
    data: (String, Vec<u8>),
    next: Address,
}

impl AddMetadata {
    fn new<S: Into<String>, A: Into<Address>>(dkey: S, dval: Vec<u8>, next: A) -> Self {
        Self {
            data: (dkey.into(), dval),
            next: next.into(),
        }
    }
}

#[ockam_core::async_trait]
impl<M: Message> SystemHandler<Context, M> for AddMetadata {
    // We implement this function only to make rustc happy.  We have
    // already been initialised fully in this test so this is not
    // required.
    async fn initialize(
        &mut self,
        _: &mut Context,
        _: &mut BTreeMap<String, Address>,
    ) -> Result<()> {
        Ok(())
    }

    async fn handle_message(
        &mut self,
        _self_addr: Address,
        ctx: &mut Context,
        msg: Routed<M>,
    ) -> Result<()> {
        info!("Handling message for AddMetadata");

        // Decode the message payload as an OckamMessage and add generic metadata to it
        let msg = OckamMessage::decode(msg.payload())?
            .generic_data(self.data.0.clone(), self.data.1.clone());

        // Then simply send the message to the next hop
        ctx.send(self.next.clone(), msg).await
    }
}

#[ockam::test]
async fn attach_metadata(ctx: &mut Context) -> Result<()> {
    let mut w = TestWorker::default();
    w.system
        .attach("worker.1", AddMetadata::new("foo", vec![42], "worker.2"));
    w.system
        .attach("worker.2", AddMetadata::new("bar", vec![7], "child")); // my favourite number

    // FIXME: @ac
    let mailboxes = Mailboxes::new(
        Mailbox::new("worker", Arc::new(AllowAll), Arc::new(AllowAll)),
        vec![
            Mailbox::new("worker.1", Arc::new(AllowAll), Arc::new(AllowAll)),
            Mailbox::new("worker.2", Arc::new(AllowAll), Arc::new(AllowAll)),
        ],
    );

    // Start the worker with three publicly mapped addresses
    WorkerBuilder::with_mailboxes(mailboxes, w)
        .start(ctx)
        .await?;

    // Send an OckamMessage wrapping a simple String payload.  In
    // reality this step should be performed by some utility in the
    // pipe worker (as an example)
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send("worker.1", OckamMessage::new(String::from("Hello Ockam!"))?)
        .await?;

    // Then wait for a reply and extract relevant metadata
    let msg = child_ctx.receive::<OckamMessage>().await?;
    info!("Received message metadata: '{:?}'", msg.generic);
    info!("Received message data: {}", String::decode(&msg.data)?);

    ctx.stop().await
}
