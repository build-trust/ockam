use crate::session::connection_status::ConnectionStatus;
use ockam_core::Route;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub(crate) enum StatusInternal {
    Up { ping_route: Route },
    Down,
}

impl Default for StatusInternal {
    fn default() -> Self {
        Self::Down
    }
}

impl StatusInternal {
    pub(crate) fn connection_status(&self) -> ConnectionStatus {
        match self {
            StatusInternal::Up { .. } => ConnectionStatus::Up,
            StatusInternal::Down => ConnectionStatus::Down,
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct Status {
    internal: Arc<Mutex<StatusInternal>>,
}

impl Status {
    pub(crate) fn connection_status(&self) -> ConnectionStatus {
        self.internal.lock().unwrap().connection_status()
    }

    pub(crate) fn set_up(&self, ping_route: Route) {
        *self.internal.lock().unwrap() = StatusInternal::Up { ping_route };
    }

    pub(crate) fn set_down(&self) {
        *self.internal.lock().unwrap() = StatusInternal::Down;
    }

    pub(crate) fn lock_clone(&self) -> StatusInternal {
        self.internal.lock().unwrap().clone()
    }

    pub(crate) async fn wait_until_up(&self) {
        // TODO: Probably possible to optimize to use some channel to notify about the state change,
        //  but tricky to do properly
        loop {
            let connection_status = self.internal.lock().unwrap().connection_status();

            match connection_status {
                ConnectionStatus::Down => sleep(Duration::from_millis(50)).await,
                ConnectionStatus::Up => break,
            }
        }
    }
}
