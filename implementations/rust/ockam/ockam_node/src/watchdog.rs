use crate::Context;
use ockam_core::{async_trait, Any, Result, Worker};
use std::path::PathBuf;
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
};

pub struct WatchdogSocketHandler {
    stream: UnixStream,
}

#[async_trait]
impl Worker for WatchdogSocketHandler {
    type Context = Context;
    type Message = Any;

    async fn shutdown(&mut self, _: &mut Context) -> Result<()> {
        if let Err(e) = self.stream.shutdown().await {
            warn!("failed to shutdown watchdog socket correctly: {}", e);
        }
        Ok(())
    }
}

impl WatchdogSocketHandler {
    /// This function will wait for a watchdog to connect to it and
    /// then signals shutdown state of the node.  If the node goes
    /// down, the socket is closed.
    pub async fn wait_for(ctx: &Context, socket_path: PathBuf) {
        let listener = UnixListener::bind(socket_path).unwrap();

        let new_ctx = ctx
            .new_detached("_internal.watchdog.listener")
            .await
            .expect("failed to create watchdog context trampoline");
        crate::spawn(async move {
            match listener.accept().await {
                Ok((stream, _)) => {
                    if let Err(e) = new_ctx
                        .start_worker("_internal.watchdog.socket", Self { stream })
                        .await
                    {
                        error!("failed to start watchdog socket worker: {}", e);
                    }
                }
                Err(e) => {
                    error!("failed to bind watchdog socket: {}", e);
                }
            }
        });
    }
}
