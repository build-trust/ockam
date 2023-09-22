use std::sync::{OnceLock, RwLock};
use tauri::{AppHandle, Manager, Runtime, Theme, WindowBuilder};
use tracing::trace;

static SYSTEM_THEME: OnceLock<RwLock<Theme>> = OnceLock::new();
static WINDOW_NAME: &str = "ockam-system-theme";

pub(crate) fn load_system_theme<R: Runtime>(app: &AppHandle<R>) {
    let current = {
        let w = match app.get_window(WINDOW_NAME) {
            None => WindowBuilder::new(app, WINDOW_NAME, tauri::WindowUrl::default())
                .always_on_top(false)
                .focused(false)
                .visible(false)
                .build()
                .expect("Failed to build window"),
            Some(w) => w,
        };
        let theme = w.theme().unwrap_or(Theme::Light);
        trace!(?theme, "Detected system theme");
        theme
    };
    let previous = SYSTEM_THEME.get_or_init(|| RwLock::new(current));
    previous.write().unwrap().clone_from(&current);
}

fn system_theme() -> Theme {
    *SYSTEM_THEME
        .get()
        .unwrap_or_else(|| panic!("The function 'load_system_theme' must be called first"))
        .read()
        .unwrap()
}

static ICONS: OnceLock<Vec<Icon>> = OnceLock::new();

struct Icon {
    name: String,
    light: Vec<u8>,
    dark: Vec<u8>,
}

fn icons() -> &'static Vec<Icon> {
    ICONS.get_or_init(|| {
        let icons = [
            (
                "arrow-repeat",
                include_bytes!("../icons/arrow-repeat-light.png").to_vec(),
                include_bytes!("../icons/arrow-repeat-dark.png").to_vec(),
            ),
            (
                "box-arrow-in-right",
                include_bytes!("../icons/box-arrow-in-right-light.png").to_vec(),
                include_bytes!("../icons/box-arrow-in-right-dark.png").to_vec(),
            ),
            (
                "check-lg",
                include_bytes!("../icons/check-lg-light.png").to_vec(),
                include_bytes!("../icons/check-lg-dark.png").to_vec(),
            ),
            (
                "clipboard2",
                include_bytes!("../icons/clipboard2-light.png").to_vec(),
                include_bytes!("../icons/clipboard2-dark.png").to_vec(),
            ),
            (
                "envelope",
                include_bytes!("../icons/envelope-light.png").to_vec(),
                include_bytes!("../icons/envelope-dark.png").to_vec(),
            ),
            (
                "file-earmark-text",
                include_bytes!("../icons/file-earmark-text-light.png").to_vec(),
                include_bytes!("../icons/file-earmark-text-dark.png").to_vec(),
            ),
            (
                "person",
                include_bytes!("../icons/person-light.png").to_vec(),
                include_bytes!("../icons/person-dark.png").to_vec(),
            ),
            (
                "plus-circle",
                include_bytes!("../icons/plus-circle-light.png").to_vec(),
                include_bytes!("../icons/plus-circle-dark.png").to_vec(),
            ),
            (
                "power",
                include_bytes!("../icons/power-light.png").to_vec(),
                include_bytes!("../icons/power-dark.png").to_vec(),
            ),
            (
                "share-fill",
                include_bytes!("../icons/share-fill-light.png").to_vec(),
                include_bytes!("../icons/share-fill-dark.png").to_vec(),
            ),
            (
                "trash3",
                include_bytes!("../icons/trash3-light.png").to_vec(),
                include_bytes!("../icons/trash3-dark.png").to_vec(),
            ),
            (
                "x-lg",
                include_bytes!("../icons/x-lg-light.png").to_vec(),
                include_bytes!("../icons/x-lg-dark.png").to_vec(),
            ),
        ];
        icons
            .into_iter()
            .map(|(name, light, dark)| Icon {
                name: name.to_string(),
                light,
                dark,
            })
            .collect()
    })
}

pub(crate) fn themed_icon(name: &str) -> Vec<u8> {
    let icon = icons()
        .iter()
        .find(|icon| icon.name == name)
        .unwrap_or_else(|| panic!("Icon '{name}' not found"));
    match system_theme() {
        Theme::Light => &icon.light,
        Theme::Dark => &icon.dark,
        _ => &icon.light,
    }
    .clone()
}
