use tauri::menu::{MenuBuilder, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_positioner::{Position, WindowExt};

use ockam_api::nodes::models::portal::OutletStatus;

use crate::app::AppState;

pub const SHARED_SERVICE_HEADER_MENU_ID: &str = "shared_service_header";
pub const SHARED_SERVICE_CREATE_MENU_ID: &str = "shared_service_create";
pub const SHARED_SERVICE_WINDOW_ID: &str = "shared_service_creation";

pub(crate) async fn build_shared_services_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    if !app_state.is_enrolled().await.unwrap_or(false) {
        return builder;
    };

    let builder = builder.items(&[
        &MenuItemBuilder::with_id(SHARED_SERVICE_HEADER_MENU_ID, "Shared")
            .enabled(false)
            .build(app_handle),
        &MenuItemBuilder::with_id(SHARED_SERVICE_CREATE_MENU_ID, "Create...").build(app_handle),
    ]);

    app_state
        .tcp_outlet_list()
        .await
        .iter()
        .map(|outlet| shared_service_submenu(app_handle, outlet))
        .fold(builder, |builder, submenu| builder.item(&submenu))
}

fn shared_service_submenu<R: Runtime>(
    app_handle: &AppHandle<R>,
    outlet: &OutletStatus,
) -> Submenu<R> {
    let worker_address = outlet.worker_address().unwrap();

    let outlet_info = String::from_utf8(worker_address.last().unwrap().data().to_vec())
        .unwrap_or_else(|_| worker_address.to_string());

    // NOTE: Event handler for dynamic ID is defined in crate::invitations::tray_menu module,
    // and reached via crate::app::tray_menu::fallback_for_id
    SubmenuBuilder::new(app_handle, outlet_info)
        .items(&[
            &MenuItemBuilder::new("Share")
                .id(format!("invitation-create-for-{}", outlet.socket_addr))
                .build(app_handle),
            &MenuItemBuilder::new(format!("TCP Address: {}", outlet.socket_addr))
                .id("outlet-tcp-address")
                .enabled(false)
                .build(app_handle),
        ])
        .build()
        .expect("cannot build menu for shared service")
}

/// Event listener for the "Create..." menu item
pub fn on_create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    match app.get_window(SHARED_SERVICE_WINDOW_ID) {
        None => {
            let w = tauri::WindowBuilder::new(
                app,
                SHARED_SERVICE_WINDOW_ID,
                tauri::WindowUrl::App("service".into()),
            )
            .always_on_top(true)
            .visible(false)
            .title("Share a service")
            .max_inner_size(450.0, 350.0)
            .resizable(false)
            .minimizable(false)
            .build()?;
            // TODO: ideally we should use Position::TrayCenter, but it's broken on the latest alpha
            let _ = w.move_window(Position::TopRight);
            w.show()?;

            #[cfg(debug_assertions)]
            {
                let app_state: State<AppState> = app.state();
                if app_state.browser_dev_tools() {
                    w.open_devtools();
                }
            }
        }
        Some(w) => w.set_focus()?,
    }
    Ok(())
}
