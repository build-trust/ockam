use crate::{pipe2::PipeSystem, Context, OckamError, OckamMessage};
use ockam_core::{
    compat::boxed::Box, Address, Any, Decodable, LocalMessage, Result, Route, Routed,
    TransportMessage, Worker,
};

#[allow(unused)]
pub struct PipeReceiver {
    system: PipeSystem,
    fin_addr: Address,
    init_addr: Option<Address>,
}

#[crate::worker]
impl Worker for PipeReceiver {
    type Context = Context;
    type Message = OckamMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::pipe2::CLUSTER_NAME).await?;
        if self.init_addr.is_some() {
            debug!(
                "PipeReceiver '{}' waiting for initialisation message",
                ctx.address()
            );
        }
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        debug!(
            "PipeReceiver: received message to address: {}",
            msg.msg_addr()
        );

        match (msg.msg_addr(), &self.init_addr) {
            (ref addr, Some(ref init)) if addr == init => {
                let peer_route = match msg.body().scope.get(0) {
                    Some(data) => Route::decode(data)?,
                    None => return Err(OckamError::InvalidParameter.into()),
                };
                trace!("Successfully initialised PipeReceiver!");
                ctx.send(peer_route, OckamMessage::new(Any)?).await
            }

            // Messagess addressed to the "fin" address can simply be forwarded
            (addr, _) if addr == self.fin_addr => {
                let inner: TransportMessage = msg.body().data()?;
                ctx.forward(LocalMessage::new(inner, vec![])).await
            }

            // For any other address we pass the message to the worker system
            (addr, _) => {
                // If the system is empty we can skip right to "fin"
                if self.system.is_empty() {
                    let inner: TransportMessage = msg.body().data()?;
                    ctx.forward(LocalMessage::new(inner, vec![])).await
                }
                // Otherwise we submit to the system
                else if addr == ctx.address() {
                    trace!(
                        "Initial dispatch to worker system: {:?}",
                        self.system.entrypoint()
                    );
                    if let Err(e) = self.system.dispatch_entry(ctx, msg).await {
                        error!("Dispatch entry error: {}", e);
                        return Err(e);
                    }
                    Ok(())
                } else {
                    trace!("Forwarding message to worker system: {}", addr);
                    self.system.handle_message(ctx, msg).await
                }
            }
        }
    }
}

impl PipeReceiver {
    pub fn new(system: PipeSystem, fin_addr: Address, init_addr: Option<Address>) -> Self {
        Self {
            system,
            fin_addr,
            init_addr,
        }
    }
}
