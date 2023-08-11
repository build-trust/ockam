use log::{info, warn};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::time::Duration;

use tauri::{
    async_runtime::spawn,
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};
use tokio::time::sleep;

pub(crate) const OCKAM_OPEN_URL_SOCK: &str = "/tmp/.ockam-open-url-sock";
const ONLY_WRITE_FROM_USER_PERMISSIONS: u32 = 0o200;

pub(crate) fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("linux-url")
        .setup(|app, _api| {
            //bind fails if the file already exists
            let _ = std::fs::remove_file(OCKAM_OPEN_URL_SOCK);
            let listener = UnixListener::bind(OCKAM_OPEN_URL_SOCK)
                .unwrap_or_else(|_| panic!("cannot listener on {OCKAM_OPEN_URL_SOCK}"));
            //only allow the current user to write to the socket
            std::fs::set_permissions(
                OCKAM_OPEN_URL_SOCK,
                std::fs::Permissions::from_mode(ONLY_WRITE_FROM_USER_PERMISSIONS),
            )
            .unwrap_or_else(|_| panic!("cannot set permissions on {OCKAM_OPEN_URL_SOCK}"));

            let handle = app.clone();
            spawn(async move {
                //wait a bit to let the app start
                sleep(Duration::from_millis(250)).await;

                //check if we had an ockam:// argument passed to us
                if let Some(url) = std::env::args().nth(1) {
                    if url.starts_with("ockam:") {
                        info!("received url: {}", url);
                        handle.trigger_global(crate::app::events::URL_OPEN, Some(url));
                    } else {
                        warn!("ignored argument: {}", url);
                    }
                }

                for stream in listener.incoming().flatten() {
                    let mut stream = stream;
                    let mut buffer = [0; 4096];
                    let read_bytes = stream.read(&mut buffer).unwrap();
                    if let Ok(url) = String::from_utf8(buffer[..read_bytes].to_vec()) {
                        if url.starts_with("ockam:") {
                            info!("received url: {}", url);
                            handle.trigger_global(crate::app::events::URL_OPEN, Some(url));
                        } else {
                            warn!("ignored url: {}", url);
                        }
                    }
                }
            });
            Ok(())
        })
        .build()
}
