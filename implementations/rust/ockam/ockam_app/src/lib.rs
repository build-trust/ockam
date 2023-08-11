use crate::app::{configure_tauri_plugin_log, process_application_event, setup_app, AppState};
use crate::error::Result;
use shared_service::tcp_outlet::tcp_outlet_create;

mod app;
mod enroll;
mod error;
#[cfg(feature = "invitations")]
mod invitations;
#[cfg(target_os = "linux")]
mod linux_url_plugin;
mod options;
mod platform;
#[cfg(feature = "invitations")]
mod projects;
mod shared_service;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            if let Ok(mut stream) = UnixStream::connect(linux_url_plugin::OCKAM_OPEN_URL_SOCK) {
                stream.write_all(args[0].as_bytes()).unwrap();
                stream.flush().unwrap();
                return;
            }
        }
    }

    // For now, the application only consists of a system tray with several menu items
    #[cfg_attr(not(feature = "invitations"), allow(unused_mut))]
    let mut builder = tauri::Builder::default()
        .plugin(configure_tauri_plugin_log())
        .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_positioner::init())
        .setup(move |app| setup_app(app))
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![tcp_outlet_create]);

    #[cfg(feature = "invitations")]
    {
        builder = builder.plugin(projects::plugin::init());
        builder = builder.plugin(invitations::plugin::init());
        #[cfg(target_os = "linux")]
        {
            builder = builder.plugin(linux_url_plugin::init());
        }
    }

    let mut app = builder
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application");

    platform::set_platform_activation_policy(&mut app);

    app.run(process_application_event);
}
