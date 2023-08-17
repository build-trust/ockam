use tauri::menu::{MenuBuilder, MenuItem};
use tauri::{AppHandle, Manager, Runtime, State};

use crate::app::AppState;
use crate::options::reset;

pub const RESET_MENU_ID: &str = "reset";
pub const QUIT_MENU_ID: &str = "quit";

pub(crate) async fn build_options_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();

    if app_state.is_enrolled().await {
        builder = builder.separator();
        builder = builder.item(&MenuItem::with_id(
            app_handle,
            RESET_MENU_ID,
            "Reset",
            true,
            Some("cmd+r"),
        ));
    }

    builder = builder.item(&MenuItem::with_id(
        app_handle,
        QUIT_MENU_ID,
        "Quit Ockam",
        true,
        Some("cmd+q"),
    ));

    builder
}

/// Event listener for the "Reset" menu item
/// Reset the persistent state
pub fn on_reset<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { reset(&app).await });
    Ok(())
}

/// Event listener for the "Quit" menu item
/// Quit the application when the user wants to
pub fn on_quit() -> tauri::Result<()> {
    std::process::exit(0);
}
