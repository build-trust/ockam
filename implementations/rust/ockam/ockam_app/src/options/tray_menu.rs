use tauri::menu::{IconMenuItemBuilder, MenuBuilder, MenuEvent, NativeIcon};
use tauri::{AppHandle, Icon, Manager, Runtime, State};
use tracing::error;

use crate::app::AppState;
use crate::icons::themed_icon;
use crate::options::reset;

const DOCS_MENU_ID: &str = "options_docs";
const RESET_MENU_ID: &str = "options_reset";
const QUIT_MENU_ID: &str = "options_quit";
const ERROR_MENU_ID: &str = "options_error";

pub(crate) async fn build_options_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();

    builder = builder.items(&[
        &IconMenuItemBuilder::with_id(DOCS_MENU_ID, "Documentation")
            .icon(Icon::Raw(themed_icon("file-earmark-text")))
            .accelerator("cmd+/")
            .build(app_handle),
        &IconMenuItemBuilder::with_id(RESET_MENU_ID, "Reset")
            .icon(Icon::Raw(themed_icon("arrow-repeat")))
            .accelerator("cmd+r")
            .build(app_handle),
        &IconMenuItemBuilder::with_id(QUIT_MENU_ID, "Quit Ockam")
            .icon(Icon::Raw(themed_icon("power")))
            .accelerator("cmd+q")
            .build(app_handle),
    ]);

    match app_state.is_enrolled().await {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            builder = builder.item(
                &IconMenuItemBuilder::with_id(
                    ERROR_MENU_ID,
                    "The application state is corrupted, please re-enroll, reset or quit the application",
                )
                    .native_icon(NativeIcon::Caution).build(app_handle)
            )
        }
    }

    builder
}

pub fn process_tray_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    event: &MenuEvent,
) -> tauri::Result<()> {
    match event.id.as_ref() {
        DOCS_MENU_ID => on_docs(),
        RESET_MENU_ID => on_reset(app),
        QUIT_MENU_ID => on_quit(),
        _ => Ok(()),
    }
}

/// Event listener for the "Documentation" menu item
/// Open the documentation in the browser
fn on_docs() -> tauri::Result<()> {
    let _ = open::that_detached("https://docs.ockam.io/")
        .map_err(|e| error!(%e, "Failed to open the documentation in the browser"));
    Ok(())
}

/// Event listener for the "Reset" menu item
/// Reset the persistent state
fn on_reset<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = reset(&app)
            .await
            .map_err(|e| error!(%e, "Failed to reset app"));
    });
    Ok(())
}

/// Event listener for the "Quit" menu item
/// Quit the application when the user wants to
fn on_quit() -> tauri::Result<()> {
    std::process::exit(0);
}
