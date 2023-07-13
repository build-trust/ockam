use crate::app::tray_menu::TrayMenu;
use std::sync::RwLock;

#[derive(Default)]
pub struct State {
    pub tray_menu: RwLock<TrayMenu>,
}
