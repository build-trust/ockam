use crate::enroll::{DefaultBackend, EnrollTrayMenuSection};
use crate::options::OptionsTrayMenuSection;
use crate::tcp::outlet::TcpOutletTrayMenuSection;
use crate::{enroll, options, tcp, AppHandle};
use tauri::{
    CustomMenuItem, SystemTrayEvent, SystemTrayHandle, SystemTrayMenu, SystemTrayMenuItem,
};
use tracing::error;

#[derive(Default)]
pub struct TrayMenu {
    pub enroll: EnrollTrayMenuSection,
    pub tcp: TcpOutletTrayMenuSection,
    pub options: OptionsTrayMenuSection,
}

impl TrayMenu {
    pub fn init(&self) -> Self {
        Self {
            enroll: EnrollTrayMenuSection::default(),
            tcp: TcpOutletTrayMenuSection::default(),
            options: OptionsTrayMenuSection::default(),
        }
    }

    pub fn build(&self) -> SystemTrayMenu {
        let mut tray_menu = SystemTrayMenu::new();
        tray_menu = self.enroll.build(tray_menu);
        tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);
        tray_menu = self.tcp.build(tray_menu);
        tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);
        tray_menu = self.options.build(tray_menu);
        tray_menu
    }

    pub fn refresh(&mut self, tray_handle: &SystemTrayHandle) {
        self.tcp.refresh();
        let _ = tray_handle.set_menu(self.build());
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

    pub fn id(&self) -> &str {
        &self.inner.id_str
    }

    pub fn set_enabled(&mut self, tray_handle: &SystemTrayHandle, enabled: bool) {
        let _ = tray_handle.get_item(self.id()).set_enabled(enabled);
        let mut inner = self.inner.clone();
        inner.enabled = enabled;
        self.inner = inner;
    }
}

pub trait TrayMenuSection: Default {
    fn build(&self, tray_menu: SystemTrayMenu) -> SystemTrayMenu;
    fn refresh(&mut self) {
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
pub fn process_system_tray_event(app_handle: AppHandle, event: SystemTrayEvent) {
    if let SystemTrayEvent::MenuItemClick { id, .. } = event {
        let result = match id.as_str() {
            enroll::ENROLL_MENU_ID => enroll::on_enroll(DefaultBackend, app_handle),
            tcp::outlet::TCP_OUTLET_CREATE_MENU_ID => tcp::outlet::on_create(app_handle),
            options::RESET_MENU_ID => options::on_reset(DefaultBackend, app_handle),
            options::QUIT_MENU_ID => options::on_quit(),
            _ => Ok(()),
        };
        if let Err(e) = result {
            error!("{:?}", e)
        }
    }
}
