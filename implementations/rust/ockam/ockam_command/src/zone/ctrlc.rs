use miette::{IntoDiagnostic, Result};
use once_cell::sync::OnceCell;
use std::sync::Arc;

static CTRLC_HANDLER: OnceCell<(
    tokio::sync::broadcast::Sender<()>,
    Arc<tokio::sync::broadcast::Receiver<()>>,
)> = OnceCell::new();

pub struct ZoneCtrlcHandler;

impl ZoneCtrlcHandler {
    fn get() -> (
        tokio::sync::broadcast::Sender<()>,
        Arc<tokio::sync::broadcast::Receiver<()>>,
    ) {
        CTRLC_HANDLER
            .get_or_init(|| {
                let (quit_tx, quit_rx) = tokio::sync::broadcast::channel(16);
                let _quit_tx = quit_tx.clone();
                let _ = ctrlc::try_set_handler(move || {
                    let _ = _quit_tx.send(());
                });
                (quit_tx, Arc::new(quit_rx))
            })
            .clone()
    }

    fn rx() -> tokio::sync::broadcast::Receiver<()> {
        let (quit_tx, _) = Self::get();
        quit_tx.subscribe()
    }

    pub fn send() {
        let (quit_tx, _) = Self::get();
        let _ = quit_tx.send(());
    }

    pub async fn wait_for_message() -> Result<()> {
        let mut quit_rx = Self::rx();
        quit_rx.recv().await.into_diagnostic()
    }
}
