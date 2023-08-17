#[cfg(all(debug_assertions, feature = "invitations"))]
use tauri::Manager;
use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, Wry};

use crate::app::AppState;
use crate::options::reset;

#[cfg(debug_assertions)]
pub const REFRESH_MENU_ID: &str = "refresh";
pub const RESET_MENU_ID: &str = "reset";
pub const QUIT_MENU_ID: &str = "quit";

pub(crate) async fn build_options_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    let tm = if app_state.is_enrolled().await {
        #[cfg(debug_assertions)]
        let tray_menu = tray_menu.add_item(CustomMenuItem::new(REFRESH_MENU_ID, "Dev: Refresh"));
        tray_menu.add_item(CustomMenuItem::new(RESET_MENU_ID, "Reset").accelerator("cmd+r"))
    } else {
        tray_menu
    };
    tm.add_item(CustomMenuItem::new(QUIT_MENU_ID, "Quit Ockam").accelerator("cmd+q"))
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
