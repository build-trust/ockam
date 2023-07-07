use tauri::{CustomMenuItem, SystemTray, SystemTrayMenu, SystemTrayMenuItem};

use crate::enroll::EnrollActions;
use crate::quit::QuitActions;

/// Create the system tray with all the major functions.
/// Separate groups of related functions with a native separator
pub fn create_system_tray() -> SystemTray {
    let enroll = EnrollActions::new();
    let quit = QuitActions::new();
    let tray_menu = SystemTrayMenu::new()
        .add_menu_items(vec![enroll.enroll])
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_menu_items(vec![enroll.reset, quit.quit]);

    SystemTray::new().with_menu(tray_menu)
}

/// This trait provides a way to add a list of
/// custom menu items to the SystemTray so that we
/// can define the behaviour of those items in separate modules.
trait SystemTrayMenuItems {
    fn add_menu_items(self, items: Vec<CustomMenuItem>) -> Self;
}

impl SystemTrayMenuItems for SystemTrayMenu {
    fn add_menu_items(self, items: Vec<CustomMenuItem>) -> Self {
        let mut tm = self;
        for item in items.iter() {
            tm = tm.add_item(item.clone());
        }
        tm
    }
}
