//! This crate contains the implementation of the Ockam desktop application.
//!
//! In order to run the application in development you need to execute:
//! ```sh
//! # to build the `ockam` executable in the target/debug directory
//! cargo build
//!
//! # to build the `ockam_desktop` executable in the target/debug directory and start it
//! # the overridden tauri configuration renames the package.productName value from "Ockam" to
//! # "OckamDesktop" so that we don't get any conflict with the command line executable name.
//! # However when the application is published we keep "Ockam" as a name since this will be the
//! # MacOS bundle name
//! cd implementations/rust/ockam/ockam_app; cargo tauri dev -c tauri.conf.dev.json; cd -
//!
//! ```

#[cfg(feature = "log")]
use crate::app::configure_tauri_plugin_log;
#[cfg(all(not(feature = "log"), feature = "tracing"))]
use crate::app::configure_tracing_log;
use crate::app::{process_application_event, setup, AppState};
use crate::cli::check_ockam_executable;
use crate::error::Result;
use std::process::exit;

mod app;
mod cli;
mod enroll;
mod error;
mod invitations;
mod options;
mod platform;
mod projects;
mod shared_service;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(all(feature = "tracing", not(feature = "log")))]
    configure_tracing_log();

    // Exit if there is any issue with the ockam command
    // The log messages should explain what went wrong
    if check_ockam_executable().is_err() {
        exit(-1)
    }

    // For now, the application only consists of a system tray with several menu items
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(shared_service::plugin::init())
        .plugin(projects::plugin::init())
        .plugin(invitations::plugin::init())
        .setup(setup)
        .manage(AppState::new());

    #[cfg(feature = "log")]
    {
        builder = builder.plugin(configure_tauri_plugin_log());
    }

    let mut app = builder
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application");

    platform::set_platform_activation_policy(&mut app);

    app.run(process_application_event);
}
