use crate::app::tray_menu::{TrayMenuItem, TrayMenuSection};
use crate::tcp::outlet::{tcp_outlet_create, tcp_outlet_list};
use crate::AppHandle;
use tauri::{CustomMenuItem, SystemTrayMenu};

pub const TCP_OUTLET_HEADER_MENU_ID: &str = "tcp_outlet_header";
pub const TCP_OUTLET_CREATE_MENU_ID: &str = "tcp_outlet_create";

pub struct TcpOutletTrayMenuSection {
    header: TrayMenuItem,
    create: TrayMenuItem,
    list: Vec<TrayMenuItem>,
}

impl TcpOutletTrayMenuSection {
    pub fn new() -> Self {
        Self {
            header: CustomMenuItem::new(TCP_OUTLET_HEADER_MENU_ID, "TCP Outlets")
                .disabled()
                .into(),
            create: CustomMenuItem::new(TCP_OUTLET_CREATE_MENU_ID, "Create...").into(),
            list: Self::get_tcp_outlet_list(),
        }
    }

    fn get_tcp_outlet_list() -> Vec<TrayMenuItem> {
        tcp_outlet_list()
            .unwrap_or(vec![])
            .iter()
            .map(|outlet| {
                let outlet_info = format!(
                    "{} to {}",
                    outlet.worker_address().unwrap(),
                    outlet.tcp_addr
                );
                CustomMenuItem::new(outlet_info.clone(), outlet_info).into()
            })
            .collect::<Vec<TrayMenuItem>>()
    }
}

impl Default for TcpOutletTrayMenuSection {
    fn default() -> Self {
        Self::new()
    }
}

impl TrayMenuSection for TcpOutletTrayMenuSection {
    fn build(&self, mut tray_menu: SystemTrayMenu) -> SystemTrayMenu {
        tray_menu = tray_menu
            .add_item(self.header.inner())
            .add_item(self.create.inner());
        for item in &self.list {
            tray_menu = tray_menu.add_item(item.inner());
        }
        tray_menu
    }

    fn refresh(&mut self) {
        self.list = Self::get_tcp_outlet_list();
    }
}

/// Event listener for the "Create..." menu item
pub fn on_create(app_handle: AppHandle) -> tauri::Result<()> {
    tcp_outlet_create(app_handle);
    Ok(())
}
