use crate::app::AppState;
use tauri::{AppHandle, Manager, Runtime, State, WindowBuilder};
use tauri_plugin_positioner::{Position, WindowExt};

pub(crate) fn create<R: Runtime>(
    app: &AppHandle<R>,
    builder: WindowBuilder<'_, R>,
    width: f64,
    height: f64,
) -> tauri::Result<()> {
    let w = builder
        .always_on_top(true)
        .min_inner_size(width, height)
        .max_inner_size(width, height)
        .resizable(true)
        .minimizable(false)
        .build()?;
    // TODO: ideally we should use Position::TrayCenter, but it's broken on the latest alpha
    let _ = w.move_window(Position::TopRight);
    w.show()?;

    #[cfg(debug_assertions)]
    {
        let app_state: State<AppState> = app.state();
        if app_state.browser_dev_tools() {
            w.open_devtools();
        }
    }

    Ok(())
}
