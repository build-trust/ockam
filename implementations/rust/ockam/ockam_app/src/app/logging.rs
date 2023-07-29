use tauri::plugin::TauriPlugin;
use tauri::Runtime;
use tauri_plugin_log::{Target, TargetKind};

pub fn configure_tauri_plugin_log<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_log::Builder::default()
        .level(tracing::log::LevelFilter::Debug)
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some("ockam.log".to_string()),
            }),
        ])
        .build()
}
