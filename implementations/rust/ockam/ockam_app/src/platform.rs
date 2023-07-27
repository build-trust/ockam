use tauri::App;

#[allow(unused_variables)]
pub fn set_platform_activation_policy(app: &mut App) {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}
