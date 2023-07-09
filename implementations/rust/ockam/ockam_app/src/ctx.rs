use tauri::{AppHandle, Wry};

#[derive(Clone)]
pub struct TauriCtx {
    app_handle: AppHandle<Wry>,
}

impl TauriCtx {
    pub fn new(app_handle: AppHandle<Wry>) -> Self {
        TauriCtx { app_handle }
    }

    pub fn app_handle(&self) -> &AppHandle<Wry> {
        &self.app_handle
    }
}
