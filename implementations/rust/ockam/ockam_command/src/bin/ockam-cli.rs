// use human_panic::setup_panic;
use log::{debug, info, trace, warn};
use ockam_command::{config::AppConfig, console::Console, AppError};
use std::time::Duration;

use ockam_command::command::CommandResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct App {
    console: Console,
    shutdown: Arc<AtomicBool>,
}

impl Default for App {
    fn default() -> Self {
        Self::load_environment();
        // Self::init_logging();

        Self {
            console: Console::default(),
            shutdown: Arc::new(AtomicBool::default()),
        }
    }
}

impl App {
    pub fn load_environment() {
        dotenv::dotenv().ok();
    }

    // FIXME: stderrlog depends on chrono, triggers:
    // - https://rustsec.org/advisories/RUSTSEC-2020-0159
    // - https://rustsec.org/advisories/RUSTSEC-2020-0071
    #[cfg(any())]
    pub fn init_logging() {
        setup_panic!();

        // stderrlog uses usize for verbosity instead of LevelFilter enum for some silly reason
        let mut verbosity = 2; // matches to LevelFilter::Info;

        if std::env::var("DEBUG").is_ok() {
            verbosity = 3; // Bump up to LevelFilter::Debug;
        }

        if std::env::var("TRACE").is_ok() {
            verbosity = 4; // Bump up to LevelFilter::Trace;
        }

        if let Err(e) = stderrlog::new().verbosity(verbosity).init() {
            panic!("Failed to initialize logging: {}", e);
        };
    }

    fn run(&mut self) -> Result<CommandResult, AppError> {
        let shutdown = self.shutdown.clone();

        let ctrlc_set = ctrlc::set_handler(move || {
            shutdown.store(true, Ordering::SeqCst);
        });

        if ctrlc_set.is_err() {
            warn!("Failed to set Ctrl-C handler");
        }

        AppConfig::evaluate()
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

fn main() {
    let mut app = App::default();

    let _command_result = match app.run() {
        Ok(command) => command,
        Err(error) => {
            app.console.error(&error);
            std::process::exit(exitcode::SOFTWARE)
        }
    };

    while !app.is_shutdown() {
        info!("doing stuff");
        debug!("debug");
        trace!("trace");
        std::thread::sleep(Duration::from_secs(1))
    }
}
