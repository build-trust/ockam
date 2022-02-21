use crate::{pipe2::PipeSystem, Context, OckamError, OckamMessage};
use ockam_core::{
    compat::boxed::Box, Address, Any, Decodable, LocalMessage, Result, Route, Routed,
    TransportMessage, Worker,
};

#[allow(unused)]
pub struct PipeReceiver {
    system: PipeSystem,
    api_addr: Address,
    init_addr: Option<Address>,
}

#[crate::worker]
impl Worker for PipeReceiver {
    type Context = Context;
    type Message = OckamMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::pipe2::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        match (msg.msg_addr(), &self.init_addr) {
            (ref addr, Some(ref init)) if addr == init => {
                let peer_route = match msg.body().scope.get(0) {
                    Some(data) => Route::decode(data)?,
                    None => return Err(OckamError::InvalidParameter.into()),
                };

                ctx.send(peer_route, OckamMessage::new(Any)?).await?;
                Ok(())
            }
            _ => {
                let inner: TransportMessage = msg.body().data()?;
                ctx.forward(LocalMessage::new(inner, vec![])).await
            }
        }
    }
}

impl PipeReceiver {
    pub fn new(system: PipeSystem, api_addr: Address, init_addr: Option<Address>) -> Self {
        Self {
            system,
            api_addr,
            init_addr,
        }
    }
}
