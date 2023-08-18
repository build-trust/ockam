use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use tauri::{
    async_runtime::spawn,
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::time::sleep;
use tracing::{info, warn};

pub(crate) fn open_url_sock_path() -> String {
    let runtime_directory = std::env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".to_string());
    format!("{runtime_directory}/ockam-open-url-sock")
}
const ONLY_WRITE_FROM_USER_PERMISSIONS: u32 = 0o200;

pub(crate) fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("linux-url")
        .setup(|app, _api| {
            let handle = app.clone();
            spawn(async move {
                let sock_path = &open_url_sock_path();
                //bind fails if the file already exists
                let _ = std::fs::remove_file(sock_path);
                let listener = UnixListener::bind(sock_path)
                    .unwrap_or_else(|_| panic!("cannot listener on {sock_path}"));
                //only allow the current user to write to the socket
                std::fs::set_permissions(
                    sock_path,
                    std::fs::Permissions::from_mode(ONLY_WRITE_FROM_USER_PERMISSIONS),
                )
                .unwrap_or_else(|_| panic!("cannot set permissions on {sock_path}"));

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

                while let Ok((mut stream, _)) = listener.accept().await {
                    let mut buffer = [0; 4096];
                    if let Ok(read_bytes) = stream.read(&mut buffer).await {
                        if let Ok(url) = String::from_utf8(buffer[..read_bytes].to_vec()) {
                            if url.starts_with("ockam:") {
                                info!("received url: {}", url);
                                handle.trigger_global(crate::app::events::URL_OPEN, Some(url));
                            } else {
                                warn!("ignored url: {}", url);
                            }
                        }
                    }
                    //every connection is used only once
                    drop(stream);
                }
            });
            Ok(())
        })
        .build()
}
