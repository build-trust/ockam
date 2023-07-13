use crate::enroll::DefaultBackend;
use crate::enroll::EnrollActions;
use crate::quit::QuitActions;
use crate::tcp::outlet::TcpOutletActions;
use crate::{enroll, quit, tcp, AppHandle, Result};
use tauri::SystemTrayEvent;
use tauri::{CustomMenuItem, SystemTrayMenu, SystemTrayMenuItem};
use tracing::error;

/// Create the system tray with all the major functions.
/// Separate groups of related functions with a native separator.
pub struct SystemTrayMenuBuilder {
    enroll: EnrollActions,
    tcp: TcpOutletActions,
    quit: QuitActions,
}

impl SystemTrayMenuBuilder {
    /// Create the default system tray menu with the basic elements (i.e. without list items).
    pub fn default() -> SystemTrayMenu {
        Self::new().build()
    }

    pub fn new() -> Self {
        let enroll = EnrollActions::new();
        let tcp = TcpOutletActions::new();
        let quit = QuitActions::new();
        Self { enroll, tcp, quit }
    }

    /// Create a `SystemTrayMenu` instance, adding the menu items in the expected order.
    pub fn build(self) -> SystemTrayMenu {
        SystemTrayMenu::new()
            .add_menu_items(&[self.enroll.enroll])
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_menu_items(&self.tcp.menu_items)
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_menu_items(&[self.enroll.reset, self.quit.quit])
    }

    /// Return a system tray menu with the latest state, including all list items.
    pub fn full() -> SystemTrayMenu {
        Self::get_full_menu().unwrap_or(Self::default())
    }

    fn get_full_menu() -> Result<SystemTrayMenu> {
        let enroll = EnrollActions::new();
        let tcp = TcpOutletActions::full()?;
        let quit = QuitActions::new();
        let menu = Self { enroll, tcp, quit }.build();
        Ok(menu)
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
        let mut tm = self;
        for item in items.iter() {
            tm = tm.add_item(item.clone());
        }
        tm
    }
}

/// This is the function dispatching events for the SystemTray
pub fn process_system_tray_event(app_handle: AppHandle, event: SystemTrayEvent) {
    if let SystemTrayEvent::MenuItemClick { id, .. } = event {
        let result = match id.as_str() {
            enroll::ENROLL_MENU_ID => enroll::on_enroll(DefaultBackend, app_handle),
            tcp::outlet::TCP_OUTLET_CREATE_MENU_ID => tcp::outlet::on_create(app_handle),
            enroll::RESET_MENU_ID => enroll::on_reset(DefaultBackend, app_handle),
            quit::QUIT_MENU_ID => quit::on_quit(),
            _ => Ok(()),
        };
        if let Err(e) = result {
            error!("{:?}", e)
        }
    }
}
