use tauri::menu::{Menu, MenuBuilder, MenuEvent};
use tauri::{AppHandle, Runtime};
use tracing::error;

use crate::enroll::build_enroll_section;
#[cfg(feature = "invitations")]
use crate::invitations::{self, build_invitations_section};
use crate::options::build_options_section;
use crate::shared_service::build_shared_services_section;
use crate::{enroll, options, shared_service};

pub async fn build_tray_menu<R: Runtime>(app_handle: &AppHandle<R>) -> Menu<R> {
    let mut builder = MenuBuilder::new(app_handle);

    builder = build_enroll_section(app_handle, builder).await;
    builder = build_shared_services_section(app_handle, builder).await;
    #[cfg(feature = "invitations")]
    {
        builder = build_invitations_section(app_handle, builder).await;
    }
    builder = builder.separator();
    builder = build_options_section(app_handle, builder).await;

    builder.build().expect("menu build failed")
}

/// This is the function dispatching events for the SystemTray Menu
pub fn process_system_tray_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let result = match event.id.as_ref() {
        enroll::ENROLL_MENU_ID => enroll::on_enroll(app),
        shared_service::SHARED_SERVICE_CREATE_MENU_ID => shared_service::on_create(app),
        options::RESET_MENU_ID => options::on_reset(app),
        options::QUIT_MENU_ID => options::on_quit(),
        id => fallback_for_id(app, id),
    };

    if let Err(e) = result {
        error!("{:?}", e)
    }
}

#[cfg(feature = "invitations")]
fn fallback_for_id<R: Runtime>(app: &AppHandle<R>, s: &str) -> tauri::Result<()> {
    if s.starts_with("invitation-") {
        invitations::dispatch_click_event(app, s)
    } else {
        Ok(())
    }
}

#[cfg(not(feature = "invitations"))]
fn fallback_for_id(_app: &AppHandle<Wry>, _s: &str) -> tauri::Result<()> {
    Ok(())
}
