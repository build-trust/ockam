use crate::zone::common_args::ZoneConfigArg;
use crate::Result;
use miette::{miette, IntoDiagnostic, WrapErr};
use notify::Watcher;
use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

static WATCHER_HANDLER: OnceCell<(
    tokio::sync::broadcast::Sender<String>,
    Arc<tokio::sync::broadcast::Receiver<String>>,
)> = OnceCell::new();

pub struct DirectoryWatcher {
    watcher: notify::RecommendedWatcher,
    tx: tokio::sync::broadcast::Sender<notify::Event>,
    root_dir: PathBuf,
    zone_config_file_name: String,
}

impl DirectoryWatcher {
    pub(crate) fn init() -> Result<()> {
        let (tx, rx) = Self::run()?;
        let _ = WATCHER_HANDLER.set((tx, Arc::new(rx)));
        Ok(())
    }

    fn get() -> Option<(
        tokio::sync::broadcast::Sender<String>,
        Arc<tokio::sync::broadcast::Receiver<String>>,
    )> {
        WATCHER_HANDLER.get().cloned()
    }

    fn run() -> Result<(
        tokio::sync::broadcast::Sender<String>,
        tokio::sync::broadcast::Receiver<String>,
    )> {
        let root_dir = std::env::current_dir()
            .into_diagnostic()
            .wrap_err("Failed to get current directory")?;
        let (watcher_tx, _rx) = tokio::sync::broadcast::channel(16);
        let _tx = watcher_tx.clone();
        let watcher = notify::RecommendedWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    let _ = _tx.send(event);
                }
            },
            notify::Config::default(),
        )
        .into_diagnostic()?;
        let zone_config_file_name = ZoneConfigArg::default()
            .zone_config_path()?
            .file_name()
            .ok_or_else(|| miette!("Invalid zone config file name"))?
            .to_string_lossy()
            .to_string();
        let _self = Self {
            watcher,
            tx: watcher_tx,
            root_dir,
            zone_config_file_name,
        };
        let (restart_tx, restart_rx) = tokio::sync::broadcast::channel(16);
        let _restart_tx = restart_tx.clone();
        tokio::task::spawn(async move {
            let _ = _self._run(_restart_tx).await;
        });
        Ok((restart_tx, restart_rx))
    }

    async fn _run(
        mut self,
        restart_tx: tokio::sync::broadcast::Sender<String>,
    ) -> miette::Result<()> {
        self.watcher
            .watch(self.root_dir.as_ref(), notify::RecursiveMode::Recursive)
            .into_diagnostic()?;
        let mut watcher_rx = self.tx.subscribe();

        let debounce_period = Duration::from_secs(3);
        let mut pending_message: Option<String> = None;
        let mut debounce_timer: Option<tokio::time::Instant> = None;

        let images_path = self.root_dir.join("images");
        loop {
            tokio::select! {
                event_result = watcher_rx.recv() => {
                    match event_result {
                        Ok(event) => {
                            let mut should_restart = false;

                            for path in &event.paths {
                                // Check for modifications of the zone config file
                                if path.file_name().is_some_and(|name| {
                                    name.to_string_lossy().as_ref() == self.zone_config_file_name
                                }) && path.parent().is_some_and(|parent| parent == self.root_dir)
                                {
                                    if let notify::EventKind::Modify(_) = event.kind {
                                        pending_message = Some("Detected changes in the zone config file".to_string());
                                        should_restart = true;
                                        break;
                                    }
                                }
                                // Check for creation or modification of files in the images directory
                                if path.starts_with(&images_path) {
                                    if let notify::EventKind::Create(_) | notify::EventKind::Modify(_) = event.kind
                                    {
                                        pending_message = Some("Detected changes in the images directory".to_string());
                                        should_restart = true;
                                        break;
                                    }
                                }
                            }

                            if should_restart {
                                // Reset the countdown timer when new changes are detected
                                debounce_timer = Some(tokio::time::Instant::now() + debounce_period);
                            }
                        },
                        Err(e) => {
                            warn!("Failed to receive file system event: {}", e);
                            break;
                        }
                    }
                },
                // Check if it's time to send the restart signal
                _ = async {
                    if let Some(timer) = debounce_timer {
                        tokio::time::sleep_until(timer).await;
                        Ok::<(), miette::Error>(())
                    } else {
                        // If there's no timer set, this branch will never complete
                        std::future::pending::<()>().await;
                        Ok(())
                    }
                } => {
                    if let Some(message) = pending_message.take() {
                        if let Err(e) = restart_tx.send(message) {
                            warn!(%e, "failed to send restart signal");
                            break;
                        }
                    }
                    // Reset the timer after sending
                    debounce_timer = None;
                }
            }
        }
        Ok(())
    }

    pub fn rx() -> Option<tokio::sync::broadcast::Receiver<String>> {
        if let Some((tx, _rx)) = Self::get() {
            Some(tx.subscribe())
        } else {
            None
        }
    }

    /// Returns a future that completes after receiving a file system event.
    /// If the watcher is not initialized, it will return a future that doesn't resolve.
    pub async fn wait_for_message() -> Result<String> {
        if let Some(mut rx) = Self::rx() {
            rx.recv().await.into_diagnostic()
        } else {
            std::future::pending::<Result<String>>().await
        }
    }
}
