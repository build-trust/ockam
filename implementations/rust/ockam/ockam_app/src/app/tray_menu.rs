use tauri::menu::{Menu, MenuBuilder, MenuEvent};
use tauri::{AppHandle, Runtime};
use tracing::error;

#[cfg(debug_assertions)]
use crate::app::dev_tools;
use crate::app::events::SystemTrayOnUpdatePayload;
use crate::enroll::{build_enroll_section, build_user_info_section};
use crate::invitations::{self, build_invitations_section};
use crate::options::build_options_section;
use crate::shared_service::build_shared_services_section;
use crate::{enroll, options, shared_service};

pub async fn build_tray_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    payload: Option<SystemTrayOnUpdatePayload>,
) -> Menu<R> {
    let mut builder = MenuBuilder::new(app_handle);

    builder = build_user_info_section(app_handle, builder).await;
    builder = build_enroll_section(app_handle, builder, &payload).await;
    builder = build_shared_services_section(app_handle, builder).await;
    builder = build_invitations_section(app_handle, builder).await;
    builder = build_options_section(app_handle, builder).await;
    #[cfg(debug_assertions)]
    {
        builder = dev_tools::build_developer_tools_section(app_handle, builder).await;
    }

    builder.build().expect("tray menu build failed")
}

/// This is the function dispatching events for the SystemTray Menu
pub fn process_system_tray_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let _ = enroll::process_tray_menu_event(app, &event).map_err(|e| error!("{:?}", e));
    let _ = shared_service::process_tray_menu_event(app, &event).map_err(|e| error!("{:?}", e));
    let _ = invitations::process_tray_menu_event(app, &event).map_err(|e| error!("{:?}", e));
    let _ = options::process_tray_menu_event(app, &event).map_err(|e| error!("{:?}", e));
    #[cfg(debug_assertions)]
    {
        let _ = dev_tools::process_tray_menu_event(app, &event).map_err(|e| error!("{:?}", e));
    }
}
