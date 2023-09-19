use crate::invitations::commands::accept_invitation;
use tauri::{AppHandle, Runtime};
use tracing::{debug, info};

pub const URL_OPENED: &str = "app/url/open";

pub async fn on_url_opened<R: Runtime>(app: AppHandle<R>, url: &str) -> crate::Result<()> {
    debug!(%url, "Processing ockam URL to accept invitation");
    if let Some(invitation_id) = parse_invitation_id_from_url(url) {
        accept_invitation(invitation_id, app).await?;
        info!(%url, "Ockam URL processed");
    } else {
        debug!(%url, "Ockam URL does not contain an invitation");
    }
    Ok(())
}

/// The invitation url has the following format: "ockam://invitations/accept/{invitation}".
/// This function returns the invitation id from the url.
fn parse_invitation_id_from_url(url: &str) -> Option<String> {
    let prefix = "ockam://invitations/accept/";
    url.strip_prefix(prefix)
        .map(|invitation_id| invitation_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_invitation_id_from_url_test() {
        let url = "ockam://invitations/accept/5cd301e8-43a0-45bc-bac9-4bdaba919c09";
        let invitation_id = parse_invitation_id_from_url(url);
        assert_eq!(
            invitation_id,
            Some("5cd301e8-43a0-45bc-bac9-4bdaba919c09".to_string())
        );

        let url = "ockam://invitations/INVITE/5cd301e8-43a0-45bc-bac9-4bdaba919c09";
        let invitation_id = parse_invitation_id_from_url(url);
        assert!(invitation_id.is_none());
    }
}
