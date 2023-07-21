use tauri::App;

pub fn set_platform_activation_policy(_app: &mut App) {
    // On non-macOS platforms, do nothing
}
