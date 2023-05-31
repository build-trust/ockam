mod enroll;

use enroll::enroll;
use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};

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
      .on_system_tray_event(|_app, event| match event {
          SystemTrayEvent::LeftClick {
              position: _,
              size: _,
              ..
          } => {}
          SystemTrayEvent::RightClick {
              position: _,
              size: _,
              ..
          } => {}
          SystemTrayEvent::DoubleClick {
              position: _,
              size: _,
              ..
          } => {}
          SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
              "enroll" => {
                  enroll();
              }
              "quit" => {
                  std::process::exit(0);
              }
              _ => {}
          },
          _ => {}
      })
      .invoke_handler(tauri::generate_handler![enroll])
      .build(tauri::generate_context!())
      .expect("error while running tauri application")
      .run(|_app_handle, event| {
          if let tauri::RunEvent::ExitRequested { api, .. } = event {
              api.prevent_exit();
          }
      });
}
