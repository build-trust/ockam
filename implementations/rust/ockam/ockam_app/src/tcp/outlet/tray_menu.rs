use crate::app::{TrayMenuItem, TrayMenuSection};
use crate::tcp::outlet::{tcp_outlet_create, tcp_outlet_list};
use ockam_core::async_trait;
use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, Wry};

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
            list: vec![],
        }
    }

    async fn get_tcp_outlet_list(app: &AppHandle<Wry>) -> Vec<TrayMenuItem> {
        tcp_outlet_list(app)
            .await
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

#[async_trait]
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

    async fn refresh(&mut self, app: &AppHandle<Wry>) {
        self.list = Self::get_tcp_outlet_list(app).await;
    }
}

/// Event listener for the "Create..." menu item
pub fn on_create(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { tcp_outlet_create(&app).await });
    Ok(())
}
