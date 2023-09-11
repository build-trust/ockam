use tauri::{AppHandle, Manager, Runtime};

pub const SYSTEM_TRAY_ON_UPDATE: &str = "app/system_tray/on_update";

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SystemTrayOnUpdatePayload {
    /// An optional status message that is shown when the application
    /// is waiting until the enroll process is done.
    pub enroll_status: Option<String>,
}

pub struct SystemTrayOnUpdatePayloadBuilder {
    payload: SystemTrayOnUpdatePayload,
}

impl SystemTrayOnUpdatePayloadBuilder {
    pub fn new() -> Self {
        Self {
            payload: SystemTrayOnUpdatePayload::default(),
        }
    }

    pub fn enroll_status(mut self, status_message: &str) -> Self {
        self.payload.enroll_status = Some(status_message.to_string());
        self
    }

    pub fn build(self) -> SystemTrayOnUpdatePayload {
        self.payload
    }
}

impl TryFrom<&str> for SystemTrayOnUpdatePayload {
    type Error = serde_json::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(s)
    }
}

pub fn system_tray_on_update<R: Runtime>(app: &AppHandle<R>) {
    app.trigger_global(SYSTEM_TRAY_ON_UPDATE, None);
}

pub fn system_tray_on_update_with_enroll_status<R: Runtime>(
    app: &AppHandle<R>,
    payload: &str,
) -> crate::Result<()> {
    let payload = Some(serde_json::to_string(
        &SystemTrayOnUpdatePayloadBuilder::new()
            .enroll_status(payload)
            .build(),
    )?);
    app.trigger_global(SYSTEM_TRAY_ON_UPDATE, payload);
    Ok(())
}
