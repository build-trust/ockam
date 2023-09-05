use crate::app::AppState;
use crate::options::{DEV_MENU_ID, OPEN_DEV_TOOLS_ID, REFRESH_MENU_ID};
use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime, State};

pub async fn build_developer_tools_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();

    builder.item(
        &SubmenuBuilder::new(app_handle, "Developer Tools")
            .items(&[
                &MenuItemBuilder::with_id(REFRESH_MENU_ID, "Refresh Data").build(app_handle),
                &CheckMenuItemBuilder::with_id(OPEN_DEV_TOOLS_ID, "Browser Dev Tools")
                    .enabled(true)
                    .checked(app_state.browser_dev_tools())
                    .build(app_handle),
                &MenuItemBuilder::with_id(
                    DEV_MENU_ID,
                    format!("Controller Address: {}", app_state.controller_address()),
                )
                .build(app_handle),
            ])
            .build()
            .expect("developer submenu build failed"),
    )
}

pub fn on_refresh<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.trigger_global(crate::projects::events::REFRESH_PROJECTS, None);
    app.trigger_global(crate::invitations::events::REFRESH_INVITATIONS, None);
    use crate::app::events::system_tray_on_update;
    system_tray_on_update(app);
    Ok(())
}

pub(crate) fn toggle_dev_tools<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let state: State<AppState> = app.state();
    state.set_browser_dev_tools(!state.browser_dev_tools());
    Ok(())
}
