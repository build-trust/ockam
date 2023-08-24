#[cfg(all(debug_assertions, feature = "invitations"))]
use tauri::Manager;
use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, SystemTraySubmenu, Wry};
#[cfg(target_os = "macos")]
use tauri_runtime::menu::NativeImage;
use tracing::log::error;

use crate::app::AppState;
use crate::options::reset;

#[cfg(debug_assertions)]
pub const DEV_MENU_ID: &str = "developer";
#[cfg(debug_assertions)]
pub const REFRESH_MENU_ID: &str = "refresh";
pub const RESET_MENU_ID: &str = "reset";
pub const QUIT_MENU_ID: &str = "quit";
pub const ERROR_MENU_ID: &str = "error";

pub(crate) async fn build_options_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    #[cfg(debug_assertions)]
    let tray_menu = build_developer_submenu(app_state, tray_menu);
    let tray_menu = tray_menu
        .add_item(CustomMenuItem::new(RESET_MENU_ID, "Reset").accelerator("cmd+r"))
        .add_item(CustomMenuItem::new(QUIT_MENU_ID, "Quit Ockam").accelerator("cmd+q"));

    match app_state.is_enrolled().await {
        Ok(_) => tray_menu,
        Err(e) => {
            error!("{}", e);
            let error_item = CustomMenuItem::new(
                ERROR_MENU_ID,
                "The application state is corrupted, please re-enroll, reset or quit the application",
            ).disabled();
            #[cfg(target_os = "macos")]
            let error_item = error_item.native_image(NativeImage::Caution);
            tray_menu.add_item(error_item)
        }
    }
}

fn build_developer_submenu(app_state: &AppState, tray_menu: SystemTrayMenu) -> SystemTrayMenu {
    let submenu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new(REFRESH_MENU_ID, "Refresh Data"))
        .add_item(
            CustomMenuItem::new(
                DEV_MENU_ID,
                format!("Controller Address: {}", app_state.controller_address()),
            )
            .disabled(),
        )
        .add_item(
            CustomMenuItem::new(DEV_MENU_ID, "Last Successful Poll: Not Implemented").disabled(),
        )
        .add_item(CustomMenuItem::new(DEV_MENU_ID, "Last Failed Poll: Not Implemented").disabled());

    tray_menu.add_submenu(SystemTraySubmenu::new("Developer Tools", submenu))
}

#[cfg(debug_assertions)]
pub fn on_refresh(
    #[cfg_attr(not(feature = "invitations"), allow(unused_variables))] app: &AppHandle<Wry>,
) -> tauri::Result<()> {
    #[cfg(feature = "invitations")]
    {
        app.trigger_global(crate::projects::events::REFRESH_PROJECTS, None);
        app.trigger_global(crate::invitations::events::REFRESH_INVITATIONS, None);
    }
    Ok(())
}

/// Event listener for the "Reset" menu item
/// Reset the persistent state
pub fn on_reset(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { reset(&app).await });
    Ok(())
}

/// Event listener for the "Quit" menu item
/// Quit the application when the user wants to
pub fn on_quit() -> tauri::Result<()> {
    std::process::exit(0);
}
