use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};

use enroll::enroll;

mod enroll;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let enroll_menu_item = CustomMenuItem::new("enroll".to_string(), "Enroll...");
    let quit_menu_item = CustomMenuItem::new("quit".to_string(), "Quit").accelerator("cmd+q");

    let tray_menu = SystemTrayMenu::new()
        .add_item(enroll_menu_item)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit_menu_item);
    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "enroll" => {
                    enroll();
                    app.tray_handle()
                        .get_item("enroll")
                        .set_title("Enrolled")
                        .unwrap();
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            },
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
