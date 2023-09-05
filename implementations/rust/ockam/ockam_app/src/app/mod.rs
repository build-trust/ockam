#[cfg(all(not(feature = "log"), feature = "tracing"))]
pub use self::tracing::configure_tracing_log;
#[cfg(feature = "log")]
pub use logging::configure_tauri_plugin_log;
pub use process::*;
pub use setup::*;
pub use state::*;
pub use tray_menu::*;

#[cfg(debug_assertions)]
mod dev_tools;
pub(crate) mod events;
#[cfg(feature = "log")]
mod logging;
mod process;
mod setup;
mod state;
#[cfg(all(not(feature = "log"), feature = "tracing"))]
mod tracing;
mod tray_menu;
