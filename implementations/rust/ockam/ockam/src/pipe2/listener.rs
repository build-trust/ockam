use crate::{pipe2::PipeReceiver, Context, OckamMessage, SystemBuilder};
use ockam_core::{
    compat::boxed::Box, Address, Any, Encodable, Mailbox, Mailboxes, Result, Routed, Worker,
};
use ockam_node::WorkerBuilder;

/// Listen for pipe2 handshakes and creates PipeReceiver workers
pub struct PipeListener {
    system: SystemBuilder<Context, OckamMessage>,
}

impl PipeListener {
    pub fn new(system: SystemBuilder<Context, OckamMessage>) -> Self {
        Self { system }
    }
}

#[crate::worker]
impl Worker for PipeListener {
    type Context = Context;
    type Message = OckamMessage;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        // We just assume that any messages that comes in is a
        // handshake request.  We probably want to check the metadata
        // in that OckamMessage to make sure.
        debug!(
            "Receiving pipe creation handshake from {}",
            msg.return_route()
        );

        // First we need to re-address the worker system.  This means
        // using the same SystemHandler instances and routes, but with
        // new addresses to not cause collisions on this node.
        let (init_addr, fin_addr) = (Address::random_local(), Address::random_local());
        let mut sys_builder = self.system.clone();
        sys_builder.readdress(&fin_addr);

        // Build the system and initialise the PipeReceiver worker
        let system = sys_builder.finalise(ctx).await?;
        let system_addrs = system.addresses();

        let worker = PipeReceiver::new(system, fin_addr.clone(), Some(init_addr.clone()));

        // Finally start the worker with the full set of used addresses
        // TODO: @ac
        let mut additional_mailboxes = vec![
            Mailbox::allow_all(init_addr.clone()),
            Mailbox::allow_all(fin_addr),
        ];
        for addr in system_addrs {
            additional_mailboxes.push(Mailbox::allow_all(addr.clone()));
        }
        // TODO: @ac
        let mailboxes = Mailboxes::new(
            Mailbox::allow_all(Address::random_local()),
            additional_mailboxes,
        );
        WorkerBuilder::with_mailboxes(mailboxes, worker)
            .start(ctx)
            .await?;

        // Store the return route of the request in the scope metadata section
        let ockam_msg = OckamMessage::new(Any)?.scope_data(msg.return_route().encode()?);
        ctx.send(init_addr, ockam_msg).await?;
        Ok(())
    }
}
