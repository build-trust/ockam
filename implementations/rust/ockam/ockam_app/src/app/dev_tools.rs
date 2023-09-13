use crate::app::AppState;
use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime, State};

const DEV_MENU_ID: &str = "developer";
const RESTART_MENU_ID: &str = "restart";
const REFRESH_MENU_ID: &str = "refresh";
const OPEN_DEV_TOOLS_ID: &str = "open_dev_tools";

pub async fn build_developer_tools_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    let controller_address = app_state.controller_address();

    builder.item(
        &SubmenuBuilder::new(app_handle, "Developer Tools")
            .items(&[
                &MenuItemBuilder::with_id(RESTART_MENU_ID, "Restart")
                    .accelerator("cmd+1")
                    .build(app_handle),
                &MenuItemBuilder::with_id(REFRESH_MENU_ID, "Refresh Data")
                    .accelerator("cmd+2")
                    .build(app_handle),
                &CheckMenuItemBuilder::with_id(OPEN_DEV_TOOLS_ID, "Browser Dev Tools")
                    .enabled(true)
                    .checked(app_state.browser_dev_tools())
                    .build(app_handle),
                &MenuItemBuilder::with_id(
                    DEV_MENU_ID,
                    format!("Controller Address: {}", controller_address),
                )
                .build(app_handle),
            ])
            .build()
            .expect("developer submenu build failed"),
    )
}

pub fn process_tray_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    event: &MenuEvent,
) -> tauri::Result<()> {
    match event.id.as_ref() {
        RESTART_MENU_ID => on_restart(app),
        REFRESH_MENU_ID => on_refresh(app),
        OPEN_DEV_TOOLS_ID => toggle_dev_tools(app),
        _ => Ok(()),
    }
}

fn on_restart<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.restart();
    Ok(())
}

fn on_refresh<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.trigger_global(crate::projects::events::REFRESH_PROJECTS, None);
    app.trigger_global(crate::invitations::events::REFRESH_INVITATIONS, None);
    crate::app::events::system_tray_on_update(app);
    Ok(())
}

fn toggle_dev_tools<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let state: State<AppState> = app.state();
    state.set_browser_dev_tools(!state.browser_dev_tools());
    Ok(())
}
