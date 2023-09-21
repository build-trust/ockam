use tauri::menu::{
    IconMenuItemBuilder, MenuBuilder, MenuEvent, MenuItemBuilder, NativeIcon, Submenu,
    SubmenuBuilder,
};
use tauri::{AppHandle, Icon, Manager, Runtime, State};
use tauri_plugin_positioner::{Position, WindowExt};
use tracing::error;

use ockam_api::nodes::models::portal::OutletStatus;

use crate::app::AppState;
use crate::invitations::pending_invitation_menu;
use crate::invitations::state::SyncInvitationsState;
use crate::shared_service::tcp_outlet::tcp_outlet_delete;

const SHARED_SERVICE_CREATE_MENU_ID: &str = "shared-service-create";
const SHARED_SERVICE_DELETE_MENU_ID_PREFIX: &str = "shared-service-delete-";
const SHARED_SERVICE_WINDOW_ID: &str = "shared-service-creation";

pub(crate) async fn build_shared_services_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    if !app_state.is_enrolled().await.unwrap_or(false) {
        return builder;
    };

    let mut builder = builder.items(&[
        &MenuItemBuilder::new("Your services")
            .enabled(false)
            .build(app_handle),
        &IconMenuItemBuilder::with_id(SHARED_SERVICE_CREATE_MENU_ID, "Create service")
            .icon(Icon::Raw(
                include_bytes!("../../icons/plus-circle.png").to_vec(),
            ))
            .accelerator("cmd+n")
            .build(app_handle),
    ]);

    let outlets = app_state.tcp_outlet_list().await;
    builder = if outlets.is_empty() {
        builder.item(
            &MenuItemBuilder::new("When you create a service it will appear here")
                .enabled(false)
                .build(app_handle),
        )
    } else {
        outlets
            .iter()
            .map(|outlet| shared_service_submenu(app_handle, outlet))
            .fold(builder, |builder, submenu| builder.item(&submenu))
    };

    let state: State<'_, SyncInvitationsState> = app_handle.state();
    let reader = state.read().await;
    builder = if reader.sent.is_empty() {
        builder
    } else {
        let mut submenu = SubmenuBuilder::new(app_handle, "Pending invitations");
        submenu = reader
            .sent
            .iter()
            .map(|invitation| pending_invitation_menu(app_handle, invitation))
            .fold(submenu, |builder, submenu| builder.item(&submenu));
        builder.item(
            &submenu
                .build()
                .expect("cannot build menu for pending invitations"),
        )
    };

    builder.separator()
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
            &IconMenuItemBuilder::new(format!("Serving at: {}", outlet.socket_addr))
                .enabled(false)
                .native_icon(NativeIcon::StatusAvailable)
                .build(app_handle),
            &IconMenuItemBuilder::new("Share")
                .id(format!("invitation-create-for-{}", outlet.socket_addr))
                .native_icon(NativeIcon::Share)
                .build(app_handle),
            &IconMenuItemBuilder::new("Delete")
                .id(format!(
                    "{SHARED_SERVICE_DELETE_MENU_ID_PREFIX}{}",
                    outlet.alias
                ))
                .icon(Icon::Raw(include_bytes!("../../icons/x-lg.png").to_vec()))
                .build(app_handle),
        ])
        .build()
        .expect("cannot build menu for shared service")
}

pub fn process_tray_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    event: &MenuEvent,
) -> tauri::Result<()> {
    match event.id.as_ref() {
        SHARED_SERVICE_CREATE_MENU_ID => on_create(app),
        id => {
            if let Some(alias) = id.strip_prefix(SHARED_SERVICE_DELETE_MENU_ID_PREFIX) {
                on_delete(app, alias)?;
            }
            Ok(())
        }
    }
}

/// Event listener for the "Create..." menu item
fn on_create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
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

fn on_delete<R: Runtime>(app: &AppHandle<R>, alias: &str) -> tauri::Result<()> {
    let app_handle = app.clone();
    let alias = alias.to_string();
    tauri::async_runtime::spawn(async move {
        let _ = tcp_outlet_delete(app_handle, alias)
            .await
            .map_err(|e| error!(%e, "Failed to delete TCP outlet"));
    });
    Ok(())
}
