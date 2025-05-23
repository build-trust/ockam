use once_cell::sync::OnceCell;

static CTRLC_HANDLER: OnceCell<tokio::sync::broadcast::Sender<()>> = OnceCell::new();

pub struct ClusterCtrlcHandler;

impl ClusterCtrlcHandler {
    pub fn rx() -> tokio::sync::broadcast::Receiver<()> {
        let quit_tx = CTRLC_HANDLER.get_or_init(|| {
            let (quit_tx, _quit_rx) = tokio::sync::broadcast::channel(16);
            let _quit_tx = quit_tx.clone();
            ctrlc::set_handler(move || {
                let _ = _quit_tx.send(());
            })
            .expect("Error setting exit signal handler");
            quit_tx
        });
        quit_tx.subscribe()
    }
}
