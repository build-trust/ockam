use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use ockam::identity::Identity;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::enroll::auth0::UserInfo;
use ockam_node::Context;
use ockam_vault::SigningSecret;
use serde::{Deserialize, Serialize};

const LONG_ABOUT: &str = include_str!("./static/export/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/export/after_long_help.txt");

/// Export an identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ExportCommand {
    name: Option<String>,
}

#[async_trait]
impl Command for ExportCommand {
    const NAME: &'static str = "identity export";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let identity_name = match &self.name {
            Some(name) => name.clone(),
            None => opts.state.get_default_identity_name().await?,
        };
        let enrolled_email = opts
            .state
            .get_identity_enrollment(&identity_name)
            .await?
            .and_then(|i| i.status().email().cloned());
        let enrolled_user = match enrolled_email.as_ref() {
            Some(email) => Some(opts.state.get_user(email).await?),
            None => None,
        };

        let named_identity = opts.state.get_named_identity(&identity_name).await?;
        let named_vault = opts
            .state
            .get_named_vault(&named_identity.vault_name())
            .await?;
        let vault = opts.state.make_vault(named_vault).await?;
        let identities = opts.state.make_identities(vault).await?;
        let vault = identities.vault();
        let identity = opts
            .state
            .get_identity_by_optional_name(Some(&identity_name))
            .await?;
        let signing_secret_key_handle = identities
            .identities_keys()
            .get_secret_key(&identity)
            .await?;
        let signing_secret_key = vault
            .identity_vault
            .export_key(&signing_secret_key_handle)
            .await?;
        let exported_identity = ExportedIdentity::new(
            &identity_name,
            enrolled_email,
            enrolled_user,
            identity,
            signing_secret_key,
        )?;
        let as_string = exported_identity.export()?;
        opts.terminal.to_stdout().machine(as_string).write_line()?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct ExportedIdentity {
    pub name: String,
    // Kept for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrolled_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrolled_user: Option<String>,
    pub change_history: String,
    signing_secret: String,
    signing_secret_type: u8,
}

impl ExportedIdentity {
    pub fn new(
        name: &str,
        enrolled_email: Option<EmailAddress>,
        enrolled_user: Option<UserInfo>,
        identity: Identity,
        signing_secret: SigningSecret,
    ) -> miette::Result<Self> {
        Ok(ExportedIdentity {
            name: name.to_string(),
            enrolled_email: enrolled_email.map(|e| e.to_string()),
            enrolled_user: enrolled_user
                .map(|e| serde_json::to_string(&e).into_diagnostic())
                .transpose()?,
            change_history: identity.export_as_string()?,
            signing_secret: hex::encode(signing_secret.key()),
            signing_secret_type: signing_secret.type_as_u8(),
        })
    }

    pub fn from_hex(hex: &str) -> miette::Result<Self> {
        let as_json = hex::decode(hex).into_diagnostic()?;
        serde_json::from_slice(&as_json).into_diagnostic()
    }

    pub fn export(&self) -> miette::Result<String> {
        let as_json = serde_json::to_string(&self).into_diagnostic()?;
        let as_hex = hex::encode(as_json);
        Ok(as_hex)
    }

    pub fn hex_decoded_change_history(&self) -> miette::Result<Vec<u8>> {
        hex::decode(&self.change_history).into_diagnostic()
    }

    pub fn signing_secret(&self) -> miette::Result<SigningSecret> {
        let key = hex::decode(&self.signing_secret).into_diagnostic()?;
        let key_type = self.signing_secret_type;
        SigningSecret::from_key(&key, key_type).into_diagnostic()
    }

    pub fn user_email(&self) -> miette::Result<Option<EmailAddress>> {
        if let Some(email) = &self.enrolled_email {
            Ok(Some(EmailAddress::parse(email)?))
        } else {
            Ok(None)
        }
    }

    pub fn user(&self) -> miette::Result<Option<UserInfo>> {
        if let Some(user_json) = &self.enrolled_user {
            let user: UserInfo = serde_json::from_str(user_json).into_diagnostic()?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backward_compatibility_missing_enrolled_user() -> miette::Result<()> {
        let json = r#"{"name":"test","enrolled_email":"user@example.com","change_history":"history","signing_secret":"secret123","signing_secret_type":1}"#;
        let hex_encoded = hex::encode(json);

        let deserialized = ExportedIdentity::from_hex(&hex_encoded)?;
        assert_eq!(deserialized.name, "test");
        assert_eq!(
            deserialized.enrolled_email,
            Some("user@example.com".to_string())
        );
        assert_eq!(deserialized.enrolled_user, None);
        assert_eq!(deserialized.change_history, "history");
        assert_eq!(deserialized.signing_secret, "secret123");
        assert_eq!(deserialized.signing_secret_type, 1);

        Ok(())
    }

    #[test]
    fn test_exported_identity_optional_fields_serialization() -> miette::Result<()> {
        // Only enrolled_email is defined
        let identity1 = ExportedIdentity {
            name: "id1".to_string(),
            enrolled_email: Some("user@example.com".to_string()),
            enrolled_user: None,
            change_history: "history1".to_string(),
            signing_secret: "secret1".to_string(),
            signing_secret_type: 1,
        };

        // Only enrolled_user is defined
        let identity2 = ExportedIdentity {
            name: "id2".to_string(),
            enrolled_email: None,
            enrolled_user: Some(r#"{"name":"User Name","email":"user@example.com"}"#.to_string()),
            change_history: "history2".to_string(),
            signing_secret: "secret2".to_string(),
            signing_secret_type: 1,
        };

        // Both enrolled_email and enrolled_user are defined
        let identity3 = ExportedIdentity {
            name: "id3".to_string(),
            enrolled_email: Some("user@example.com".to_string()),
            enrolled_user: Some(r#"{"name":"User Name","email":"user@example.com"}"#.to_string()),
            change_history: "history3".to_string(),
            signing_secret: "secret3".to_string(),
            signing_secret_type: 1,
        };

        for identity in [identity1, identity2, identity3] {
            let serialized = identity.export()?;
            let deserialized = ExportedIdentity::from_hex(&serialized)?;
            assert_eq!(identity.name, deserialized.name);
            assert_eq!(identity.enrolled_email, deserialized.enrolled_email);
            assert_eq!(identity.enrolled_user, deserialized.enrolled_user);
            assert_eq!(identity.change_history, deserialized.change_history);
            assert_eq!(identity.signing_secret, deserialized.signing_secret);
            assert_eq!(
                identity.signing_secret_type,
                deserialized.signing_secret_type
            );
        }

        Ok(())
    }
}
