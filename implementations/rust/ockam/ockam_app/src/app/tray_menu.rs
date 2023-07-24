use tauri::{AppHandle, CustomMenuItem, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, Wry};
use tracing::error;

use ockam_core::async_trait;

use crate::app::AppState;
use crate::enroll::EnrollTrayMenuSection;
use crate::options::OptionsTrayMenuSection;
use crate::tcp::outlet::TcpOutletTrayMenuSection;
use crate::{enroll, options, tcp};

#[derive(Default)]
pub struct TrayMenu {
    pub enroll: EnrollTrayMenuSection,
    pub tcp: TcpOutletTrayMenuSection,
    pub options: OptionsTrayMenuSection,
}

impl TrayMenu {
    pub fn build(&self, is_enrolled: bool) -> SystemTrayMenu {
        let mut tray_menu = SystemTrayMenu::new();
        if is_enrolled {
            tray_menu = self.tcp.build(tray_menu);
            tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);
            tray_menu = self.options.build(tray_menu);
        } else {
            tray_menu = self.enroll.build(tray_menu);
            tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);
            tray_menu = self.options.build(tray_menu);
        }
        tray_menu
    }

    pub async fn refresh(&mut self, app: &AppHandle<Wry>, app_state: &AppState) {
        self.enroll.refresh(app_state).await;
        self.tcp.refresh(app_state).await;
        self.options.refresh(app_state).await;
        let _ = app
            .tray_handle()
            .set_menu(self.build(app_state.is_enrolled()));
    }
}

pub struct TrayMenuItem {
    pub inner: CustomMenuItem,
}

impl From<CustomMenuItem> for TrayMenuItem {
    fn from(item: CustomMenuItem) -> Self {
        Self { inner: item }
    }
}

impl TrayMenuItem {
    pub fn inner(&self) -> CustomMenuItem {
        self.inner.clone()
    }
}

#[async_trait]
pub trait TrayMenuSection {
    fn build(&self, tray_menu: SystemTrayMenu) -> SystemTrayMenu;

    async fn refresh(&mut self, _app: &AppState) {
        // Do nothing by default
    }
}

/// This trait provides a way to add a list of
/// custom menu items to the SystemTray so that we
/// can define the behaviour of those items in separate modules.
pub(crate) trait SystemTrayMenuItems {
    fn add_menu_items(self, items: &[CustomMenuItem]) -> Self;
}

impl SystemTrayMenuItems for SystemTrayMenu {
    fn add_menu_items(self, items: &[CustomMenuItem]) -> Self {
        items
            .iter()
            .fold(self, |menu, item| menu.add_item(item.clone()))
    }
}

/// This is the function dispatching events for the SystemTray
pub fn process_system_tray_event(app: &AppHandle<Wry>, event: SystemTrayEvent) {
    if let SystemTrayEvent::MenuItemClick { id, .. } = event {
        let result = match id.as_str() {
            enroll::ENROLL_MENU_ID => enroll::on_enroll(app),
            tcp::outlet::TCP_OUTLET_CREATE_MENU_ID => tcp::outlet::on_create(app),
            options::RESET_MENU_ID => options::on_reset(app),
            options::QUIT_MENU_ID => options::on_quit(),
            _ => Ok(()),
        };
        if let Err(e) = result {
            error!("{:?}", e)
        }
    }
}
