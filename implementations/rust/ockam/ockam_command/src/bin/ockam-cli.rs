use log::{error, warn};
use ockam_command::{config::AppConfig, AppError};
use std::time::Duration;

use human_panic::setup_panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct App {
    shutdown: Arc<AtomicBool>,
}

impl Default for App {
    fn default() -> Self {
        Self::load_environment();
        Self::init_logging();

        Self {
            shutdown: Arc::new(AtomicBool::default()),
        }
    }
}

impl App {
    pub fn load_environment() {
        dotenv::dotenv().ok();
    }

    pub fn init_logging() {
        setup_panic!();

        // TODO: Clashing with ockam logging
        // env_logger::init();
    }

    async fn run(&mut self, mut ctx: ockam::Context) -> Result<(), AppError> {
        let shutdown = self.shutdown.clone();

        let ctrlc_set = ctrlc::set_handler(move || {
            shutdown.store(true, Ordering::SeqCst);
        });

        if ctrlc_set.is_err() {
            warn!("Failed to set Ctrl-C handler");
        }

        AppConfig::evaluate(&mut ctx).await?;

        while !self.is_shutdown() {
            std::thread::sleep(Duration::from_secs(1))
        }

        ctx.stop().await?;
        Ok(())
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

#[ockam::node]
async fn main(ctx: ockam::Context) {
    let mut app = App::default();
    if let Err(error) = app.run(ctx).await {
        error!("The application has encountered an error: {:?}", error)
    }
}
