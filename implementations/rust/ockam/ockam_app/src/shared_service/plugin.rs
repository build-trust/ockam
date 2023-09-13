use crate::shared_service::tcp_outlet::*;

use tauri::{
    plugin::{Builder, TauriPlugin},
    Wry,
};

pub(crate) fn init() -> TauriPlugin<Wry> {
    Builder::new("shared_service")
        .invoke_handler(tauri::generate_handler![
            tcp_outlet_create,
            tcp_outlet_delete
        ])
        .build()
}
