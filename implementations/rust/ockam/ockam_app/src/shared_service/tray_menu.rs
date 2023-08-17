use tauri::menu::{MenuBuilder, MenuItem, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::app::AppState;
use ockam_api::nodes::models::portal::OutletStatus;

pub const SHARED_SERVICE_HEADER_MENU_ID: &str = "shared_service_header";
pub const SHARED_SERVICE_CREATE_MENU_ID: &str = "shared_service_create";
pub const SHARED_SERVICE_WINDOW_ID: &str = "shared_service_creation";

pub(crate) async fn build_shared_services_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    if !app_state.is_enrolled().await {
        return builder;
    };

    for outlet in app_state.tcp_outlet_list().await {
        builder = builder.item(&shared_service_submenu(&outlet, app_handle))
    }

    builder.separator().items(&[
        &MenuItem::with_id(
            app_handle,
            SHARED_SERVICE_HEADER_MENU_ID,
            "Shared",
            false,
            None,
        ),
        &MenuItem::with_id(
            app_handle,
            SHARED_SERVICE_CREATE_MENU_ID,
            "Create...",
            true,
            None,
        ),
    ])
}

fn shared_service_submenu<R: Runtime>(
    outlet: &OutletStatus,
    app_handle: &AppHandle<R>,
) -> Submenu<R> {
    let worker_address = outlet.worker_address().unwrap();

    let outlet_info = format!("{} to {}", worker_address, outlet.tcp_addr);
    let mut submenu = SubmenuBuilder::new(app_handle, outlet_info);

    #[cfg(feature = "invitations")]
    {
        // NOTE: Event handler for dynamic ID is defined in crate::invitations::tray_menu module,
        // and reached via crate::app::tray_menu::fallback_for_id
        submenu = submenu.item(&MenuItem::with_id(
            app_handle,
            format!("invitation-create-for-{}", outlet.tcp_addr),
            "Share".to_string(),
            true,
            None,
        ));
    }

    submenu = submenu.items(&[
        &MenuItem::with_id(
            app_handle,
            "outlet-tcp-address".to_string(),
            format!("TCP Address: {}", outlet.tcp_addr),
            false,
            None,
        ),
        &MenuItem::with_id(
            app_handle,
            "outlet-worker-address".to_string(),
            format!("Worker Address: {}", worker_address),
            false,
            None,
        ),
        &MenuItem::with_id(
            app_handle,
            "outlet-worker-status".to_string(),
            format!("Status: {}", "unknown"),
            false,
            None,
        ),
    ]);

    submenu.build().expect("Failed to build outlet submenu")
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
        }
        Some(w) => w.show()?,
    }
    Ok(())
}
