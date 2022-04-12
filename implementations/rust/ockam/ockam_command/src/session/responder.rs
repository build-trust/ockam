use crate::session::error::SessionManagementError;
use crate::session::msg::SessionMsg;
use ockam::{Context, Result, Routed, Worker};
use tracing::info;

pub struct SessionResponder;

#[ockam::worker]
impl Worker for SessionResponder {
    type Message = SessionMsg;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        match msg.body() {
            SessionMsg::Ping(request_id) => {
                info!("Received keep-alive ping {}, sending pong", request_id);
                ctx.send(return_route, SessionMsg::Pong(request_id)).await?
            }
            SessionMsg::Pong(_) | SessionMsg::Heartbeat => {
                return Err(SessionManagementError::MismatchedRequestType.into());
            }
        }

        Ok(())
    }
}
