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
use crate::error::{Error, Result};
use std::process::exit;

mod app;
pub(crate) mod background_node;
mod cli;
mod enroll;
mod error;
pub(crate) mod icons;
mod invitations;
mod ockam_url;
mod options;
mod platform;
mod projects;
mod shared_service;
pub(crate) mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(all(feature = "tracing", not(feature = "log")))]
    configure_tracing_log();

    // Exit if there is any issue with the ockam command
    // The log messages should explain what went wrong
    if check_ockam_executable().is_err() {
        exit(-1)
    }

    //On linux only, to handle ockam:// link argument from args we check if
    //the app is already running, send a packet to it via unix socket.
    //Using unix socket rather than ockam, to fully separate concerns.
    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        use std::os::unix::net::UnixStream;

        let mut args = std::env::args();
        args.next(); //skip the first argument which is the executable name
        let args: Vec<String> = args.collect();

        //if we can connect to the socket then the app is already running
        //if it's not running yet the arguments will be checked upon startup
        if !args.is_empty() && args[0].starts_with("ockam:") {
            if let Ok(mut stream) =
                UnixStream::connect(ockam_url::plugin::linux::open_url_sock_path())
            {
                stream.write_all(args[0].as_bytes()).unwrap();
                stream.flush().unwrap();
                return;
            }
        }
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
        .plugin(ockam_url::plugin::init())
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
