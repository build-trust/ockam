use core::fmt;
use core::fmt::Write;
use std::fmt::Formatter;

use cli_table::{Cell, Style, Table};
use colorful::Colorful;
use miette::miette;
use miette::IntoDiagnostic;
use minicbor::Encode;
use ockam::identity::models::{
    CredentialAndPurposeKey, CredentialData, CredentialVerifyingKey, PurposeKeyAttestation,
    PurposeKeyAttestationData, PurposePublicKey,
};
use ockam::identity::{Credential, Identifier, Identity, TimestampInSeconds};
use serde::{Serialize, Serializer};

use ockam_api::cli_state::{ProjectConfigCompact, StateItemTrait, VaultState};
use ockam_api::cloud::project::Project;
use ockam_api::cloud::space::Space;
use ockam_api::nodes::models::portal::{InletStatus, OutletStatus};
use ockam_api::nodes::models::secure_channel::{
    CreateSecureChannelResponse, ShowSecureChannelResponse,
};
use ockam_api::route_to_multiaddr;
use ockam_core::api::Reply;
use ockam_core::{route, Route};
use ockam_vault::{
    ECDSASHA256CurveP256PublicKey, EdDSACurve25519PublicKey, VerifyingPublicKey, X25519PublicKey,
};

use crate::terminal::OckamColor;
use crate::util::comma_separated;
use crate::Result;

/// Trait to control how a given type will be printed as a CLI output.
///
/// The `Output` allows us to reuse the same formatting logic across different commands
/// and extract the formatting logic out of the commands logic.
///
/// Note that we can't just implement the `Display` trait because most of the types we want
/// to output in the commands are defined in other crates. We can still reuse the `Display`
/// implementation if it's available and already formats the type as we want. For example:
///
/// ```ignore
/// struct MyType;
///
/// impl std::fmt::Display for MyType {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "MyType")
///     }
/// }
///
/// impl Output for MyType {
///     fn output(&self) -> Result<String> {
///         Ok(self.to_string())
///     }
/// }
/// ```
pub trait Output {
    fn output(&self) -> Result<String>;

    fn list_output(&self) -> Result<String> {
        self.output()
    }
}

impl<O: Output> Output for &O {
    fn output(&self) -> Result<String> {
        (*self).output()
    }
}

impl Output for String {
    fn output(&self) -> Result<String> {
        Ok(self.clone())
    }
}

impl Output for &str {
    fn output(&self) -> Result<String> {
        Ok(self.to_string())
    }
}

impl Output for Space {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Space").into_diagnostic()?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Name: {}", self.name)?;
        write!(w, "\n  Users: {}", comma_separated(&self.users))?;
        Ok(w)
    }

    fn list_output(&self) -> Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "Space {}",
            self.name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        writeln!(
            output,
            "Id {}",
            self.id
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        write!(output, "{}", comma_separated(&self.users))?;

        Ok(output)
    }
}

impl Output for Vec<Space> {
    fn output(&self) -> Result<String> {
        if self.is_empty() {
            return Ok("No spaces found".to_string());
        }
        let mut rows = vec![];
        for Space {
            id, name, users, ..
        } in self
        {
            rows.push([id.cell(), name.cell(), comma_separated(users).cell()]);
        }
        let table = rows
            .table()
            .title([
                "Id".cell().bold(true),
                "Name".cell().bold(true),
                "Users".cell().bold(true),
            ])
            .display()?
            .to_string();
        Ok(table)
    }
}

impl Output for Project {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Project")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Name: {}", self.name)?;
        write!(w, "\n  Access route: {}", self.access_route)?;
        write!(
            w,
            "\n  Identity identifier: {}",
            self.identity
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_default()
        )?;
        write!(
            w,
            "\n  Version: {}",
            self.version.as_deref().unwrap_or("N/A")
        )?;
        write!(w, "\n  Running: {}", self.running.unwrap_or(false))?;
        Ok(w)
    }

    fn list_output(&self) -> Result<String> {
        let output = format!(
            r#"Project {}
Space {}"#,
            self.name
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.space_name
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
        );

        Ok(output)
    }
}

impl Output for ProjectConfigCompact {
    fn output(&self) -> Result<String> {
        let pi = self
            .identity
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let ar = self.authority_access_route.as_deref().unwrap_or("N/A");
        let ai = self.authority_identity.as_deref().unwrap_or("N/A");
        let mut w = String::new();
        writeln!(w, "{}: {}", "Project ID".bold(), self.id)?;
        writeln!(w, "{}: {}", "Project identity".bold(), pi)?;
        writeln!(w, "{}: {}", "Authority address".bold(), ar)?;
        write!(w, "{}: {}", "Authority identity".bold(), ai)?;
        Ok(w)
    }
}

impl Output for Vec<Project> {
    fn output(&self) -> Result<String> {
        if self.is_empty() {
            return Ok("No projects found".to_string());
        }
        let mut rows = vec![];
        for Project {
            id,
            name,
            users,
            space_name,
            ..
        } in self
        {
            rows.push([
                id.cell(),
                name.cell(),
                comma_separated(users).cell(),
                space_name.cell(),
            ]);
        }
        let table = rows
            .table()
            .title([
                "Id".cell().bold(true),
                "Name".cell().bold(true),
                "Users".cell().bold(true),
                "Space Name".cell().bold(true),
            ])
            .display()?
            .to_string();
        Ok(table)
    }
}

impl Output for CreateSecureChannelResponse {
    fn output(&self) -> Result<String> {
        let addr = route_to_multiaddr(&route![self.addr.to_string()])
            .ok_or(miette!("Invalid Secure Channel Address"))?
            .to_string();
        Ok(addr)
    }
}

impl Output for ShowSecureChannelResponse {
    fn output(&self) -> Result<String> {
        let s = match &self.channel {
            Some(addr) => {
                format!(
                    "\n  Secure Channel:\n{} {}\n{} {}\n{} {}",
                    "  •         At: ".light_magenta(),
                    route_to_multiaddr(&route![addr.to_string()])
                        .ok_or(miette!("Invalid Secure Channel Address"))?
                        .to_string()
                        .light_yellow(),
                    "  •         To: ".light_magenta(),
                    self.route.clone().unwrap().light_yellow(),
                    "  • Authorized: ".light_magenta(),
                    self.authorized_identifiers
                        .as_ref()
                        .unwrap_or(&vec!["none".to_string()])
                        .iter()
                        .map(|id| id.clone().light_yellow().to_string())
                        .collect::<Vec<String>>()
                        .join("\n\t")
                )
            }
            None => format!("{}", "Channel not found".red()),
        };

        Ok(s)
    }
}

impl Output for OutletStatus {
    fn output(&self) -> Result<String> {
        let output = format!(
            r#"
Outlet {}:
    TCP Address:    {}
    Worker Address: {}
"#,
            self.alias,
            self.socket_addr,
            self.worker_address()?
        );

        Ok(output)
    }

    fn list_output(&self) -> Result<String> {
        let output = format!(
            r#"Outlet {}
From {} to {}"#,
            self.alias
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.worker_address()?
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.socket_addr
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
        );

        Ok(output)
    }
}

impl Output for Vec<u8> {
    fn output(&self) -> Result<String> {
        Ok(hex::encode(self))
    }
}

impl Output for InletStatus {
    fn output(&self) -> Result<String> {
        let outlet = if let Some(r) = Route::parse(&self.outlet_route) {
            if let Some(ma) = route_to_multiaddr(&r) {
                ma.to_string()
            } else {
                self.outlet_route.to_string()
            }
        } else {
            self.outlet_route.to_string()
        };

        let output = format!(
            r#"
Inlet {}
    TCP Address: {}
    Outlet Address: {}
            "#,
            self.alias
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.bind_addr
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            outlet.color(OckamColor::PrimaryResource.color())
        );

        Ok(output)
    }

    fn list_output(&self) -> Result<String> {
        let output = format!(
            r#"Inlet {}
From {} to {}"#,
            self.alias
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.bind_addr
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.outlet_route
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
        );

        Ok(output)
    }
}

impl Output for VaultState {
    fn output(&self) -> Result<String> {
        let mut output = String::new();
        writeln!(output, "Name: {}", self.name())?;
        writeln!(
            output,
            "Type: {}",
            match self.config().is_aws() {
                true => "AWS KMS",
                false => "OCKAM",
            }
        )?;
        Ok(output)
    }

    fn list_output(&self) -> Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "Vault {}",
            self.name().color(OckamColor::PrimaryResource.color())
        )?;
        write!(
            output,
            "Type {}",
            match self.config().is_aws() {
                true => "AWS KMS",
                false => "OCKAM",
            }
            .to_string()
            .color(OckamColor::PrimaryResource.color())
        )?;
        Ok(output)
    }
}

fn human_readable_time(time: TimestampInSeconds) -> String {
    use time::format_description::well_known::iso8601::*;
    use time::Error::Format;
    use time::OffsetDateTime;

    match OffsetDateTime::from_unix_timestamp(*time as i64) {
        Ok(time) => {
            match time.format(
                &Iso8601::<
                    {
                        Config::DEFAULT
                            .set_time_precision(TimePrecision::Second {
                                decimal_digits: None,
                            })
                            .encode()
                    },
                >,
            ) {
                Ok(now_iso) => now_iso,
                Err(_) => {
                    Format(time::error::Format::InvalidComponent("timestamp error")).to_string()
                }
            }
        }
        Err(_) => Format(time::error::Format::InvalidComponent(
            "unix time is invalid",
        ))
        .to_string(),
    }
}

pub struct X25519PublicKeyDisplay(pub X25519PublicKey);

impl fmt::Display for X25519PublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "X25519: {}", hex::encode(self.0 .0))
    }
}

pub struct Ed25519PublicKeyDisplay(pub EdDSACurve25519PublicKey);

impl fmt::Display for Ed25519PublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519: {}", hex::encode(self.0 .0))
    }
}

pub struct P256PublicKeyDisplay(pub ECDSASHA256CurveP256PublicKey);

impl fmt::Display for P256PublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "P256: {}", hex::encode(self.0 .0))
    }
}

pub struct PurposePublicKeyDisplay(pub PurposePublicKey);

impl fmt::Display for PurposePublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            PurposePublicKey::SecureChannelStatic(key) => {
                writeln!(
                    f,
                    "Secure Channel Key -> {}",
                    X25519PublicKeyDisplay(key.clone())
                )?;
            }
            PurposePublicKey::CredentialSigning(key) => match key {
                CredentialVerifyingKey::EdDSACurve25519(key) => {
                    writeln!(
                        f,
                        "Credentials Key -> {}",
                        Ed25519PublicKeyDisplay(key.clone())
                    )?;
                }
                CredentialVerifyingKey::ECDSASHA256CurveP256(key) => {
                    writeln!(
                        f,
                        "Credentials Key -> {}",
                        P256PublicKeyDisplay(key.clone())
                    )?;
                }
            },
        }

        Ok(())
    }
}

pub struct CredentialDisplay(pub Credential);

impl fmt::Display for CredentialDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let versioned_data = match self.0.get_versioned_data() {
            Ok(versioned_data) => versioned_data,
            Err(_) => {
                writeln!(f, "Invalid VersionedData")?;
                return Ok(());
            }
        };

        writeln!(f, "Version:                    {}", versioned_data.version)?;

        let credential_data = match CredentialData::get_data(&versioned_data) {
            Ok(credential_data) => credential_data,
            Err(_) => {
                writeln!(f, "Invalid CredentialData")?;
                return Ok(());
            }
        };

        if let Some(subject) = &credential_data.subject {
            writeln!(f, "Subject:                    {}", subject)?;
        }

        if let Some(subject_latest_change_hash) = &credential_data.subject_latest_change_hash {
            writeln!(
                f,
                "Subject Latest Change Hash: {}",
                subject_latest_change_hash
            )?;
        }

        writeln!(
            f,
            "Created:                    {}",
            human_readable_time(credential_data.created_at)
        )?;
        writeln!(
            f,
            "Expires:                    {}",
            human_readable_time(credential_data.expires_at)
        )?;

        writeln!(f, "Attributes: ")?;

        write!(
            f,
            "  Schema: {}; ",
            credential_data.subject_attributes.schema.0
        )?;

        f.debug_map()
            .entries(credential_data.subject_attributes.map.iter().map(|(k, v)| {
                (
                    std::str::from_utf8(k).unwrap_or("**binary**"),
                    std::str::from_utf8(v).unwrap_or("**binary**"),
                )
            }))
            .finish()?;

        Ok(())
    }
}

pub struct PurposeKeyDisplay(pub PurposeKeyAttestation);

impl fmt::Display for PurposeKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let versioned_data = match self.0.get_versioned_data() {
            Ok(versioned_data) => versioned_data,
            Err(_) => {
                writeln!(f, "Invalid VersionedData")?;
                return Ok(());
            }
        };

        writeln!(f, "Version:                    {}", versioned_data.version)?;

        let purpose_key_attestation_data =
            match PurposeKeyAttestationData::get_data(&versioned_data) {
                Ok(purpose_key_attestation_data) => purpose_key_attestation_data,
                Err(_) => {
                    writeln!(f, "Invalid PurposeKeyAttestationData")?;
                    return Ok(());
                }
            };

        writeln!(
            f,
            "Subject:                    {}",
            purpose_key_attestation_data.subject
        )?;

        writeln!(
            f,
            "Subject Latest Change Hash: {}",
            purpose_key_attestation_data.subject_latest_change_hash
        )?;

        writeln!(
            f,
            "Created:                    {}",
            human_readable_time(purpose_key_attestation_data.created_at)
        )?;
        writeln!(
            f,
            "Expires:                    {}",
            human_readable_time(purpose_key_attestation_data.expires_at)
        )?;

        writeln!(
            f,
            "Public Key -> {}",
            PurposePublicKeyDisplay(purpose_key_attestation_data.public_key.clone())
        )?;

        Ok(())
    }
}

#[derive(Encode)]
#[cbor(transparent)]
pub struct CredentialAndPurposeKeyDisplay(#[n(0)] pub CredentialAndPurposeKey);

impl Output for CredentialAndPurposeKeyDisplay {
    fn output(&self) -> Result<String> {
        Ok(format!("{}", self))
    }
}

impl fmt::Display for CredentialAndPurposeKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // TODO: Could borrow using a lifetime
        writeln!(f, "Credential:")?;
        writeln!(f, "{}", CredentialDisplay(self.0.credential.clone()))?;
        writeln!(f)?;
        writeln!(f, "Purpose key:")?;
        writeln!(
            f,
            "{}",
            PurposeKeyDisplay(self.0.purpose_key_attestation.clone())
        )?;

        Ok(())
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct IdentifierDisplay(pub Identifier);

impl fmt::Display for IdentifierDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Output for IdentifierDisplay {
    fn output(&self) -> Result<String> {
        Ok(self.to_string())
    }
}

pub struct IdentityDisplay(pub Identity);

impl Serialize for IdentityDisplay {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        serializer.serialize_bytes(&self.0.export().map_err(Error::custom)?)
    }
}

pub struct VerifyingPublicKeyDisplay(pub VerifyingPublicKey);

impl fmt::Display for VerifyingPublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            VerifyingPublicKey::EdDSACurve25519(value) => {
                write!(f, "EdDSACurve25519: {}", hex::encode(value.0))
            }
            VerifyingPublicKey::ECDSASHA256CurveP256(value) => {
                write!(f, "ECDSASHA256CurveP256: {}", hex::encode(value.0))
            }
        }
    }
}

impl Serialize for VerifyingPublicKeyDisplay {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&match &self.0 {
            VerifyingPublicKey::EdDSACurve25519(value) => {
                format!("EdDSACurve25519: {}", hex::encode(value.0))
            }
            VerifyingPublicKey::ECDSASHA256CurveP256(value) => {
                format!("ECDSASHA256CurveP256: {}", hex::encode(value.0))
            }
        })
    }
}

impl fmt::Display for IdentityDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Identifier: {}", self.0.identifier())?;
        for (i_num, change) in self.0.changes().iter().enumerate() {
            writeln!(f, "  Change[{}]:", i_num)?;
            writeln!(
                f,
                "    identifier:              {}",
                hex::encode(change.change_hash())
            )?;
            writeln!(
                f,
                "    primary_public_key:      {}",
                VerifyingPublicKeyDisplay(change.primary_public_key().clone())
            )?;
            writeln!(
                f,
                "    revoke_all_purpose_keys: {}",
                change.data().revoke_all_purpose_keys
            )?;
        }

        Ok(())
    }
}

impl Output for IdentityDisplay {
    fn output(&self) -> Result<String> {
        Ok(format!("{}", self))
    }
}

impl<T: Output> Output for Reply<T> {
    fn output(&self) -> Result<String> {
        match self {
            Reply::Successful(t) => t.output(),
            Reply::Failed(e, status) => {
                let mut output = String::new();
                if let Some(m) = e.message() {
                    writeln!(output, "Failed request: {m}")?;
                } else {
                    writeln!(output, "Failed request")?;
                };
                if let Some(status) = status {
                    writeln!(output, "status: {status}")?;
                }
                Ok(output)
            }
        }
    }
}
