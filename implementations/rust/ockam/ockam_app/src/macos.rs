use tauri::App;

pub fn set_platform_activation_policy(app: &mut App) {
    // macOS-specific implementation for setting the activation policy
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}