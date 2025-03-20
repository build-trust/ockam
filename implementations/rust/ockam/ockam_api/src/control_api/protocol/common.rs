use ockam::identity::{Identity, Vault};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use utoipa::openapi::{ObjectBuilder, OneOfBuilder, RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

// This is an alias for documentation purposes only
/// The destination node name.
/// Depending on the type of node resolution used, it can be a relay name
/// or a dns address.
/// The special value `self` can be used to refer to the current node.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct NodeName(String);

// This is an alias for documentation purposes only
#[derive(Serialize, Deserialize, ToSchema, Debug)]
#[schema(
    description =
r#"
Credential attributes.
Attributes are key-value pairs that can be used to describe a credential.
Ockam uses `ockam-` as a prefix for its own attributes.
[You can learn more about attributes in the Ockam documentation](https://docs.ockam.io/reference/protocols/access-controls)
"#,
    example = json!({
        "ockam-role": "member",
        "ockam-relay": "relay-name",
        "my-attribute": "my-value",
    })
)]
pub struct Attributes(pub BTreeMap<String, String>);

#[derive(Debug)]
pub struct HostnamePort {
    pub hostname: String,
    pub port: u16,
}

impl PartialSchema for HostnamePort {
    fn schema() -> RefOr<Schema> {
        RefOr::T(Schema::OneOf(
            OneOfBuilder::new()
                .item(RefOr::T(Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(utoipa::openapi::schema::Type::String)
                        .format(Some(utoipa::openapi::SchemaFormat::Custom(
                            "hostname:port".to_string(),
                        )))
                        .build(),
                )))
                .item(
                    // Object to support {hostname: "example.com", port: 8080}
                    RefOr::T(Schema::Object(
                        ObjectBuilder::new()
                            .schema_type(utoipa::openapi::schema::Type::Object)
                            .property(
                                "hostname",
                                ObjectBuilder::new()
                                    .schema_type(utoipa::openapi::schema::Type::String)
                                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(
                                        utoipa::openapi::KnownFormat::Hostname,
                                    )))
                                    .build(),
                            )
                            .property(
                                "port",
                                ObjectBuilder::new()
                                    .schema_type(utoipa::openapi::schema::Type::Integer)
                                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(
                                        utoipa::openapi::KnownFormat::Int32,
                                    )))
                                    .build(),
                            )
                            .build(),
                    )),
                )
                .build(),
        ))
    }
}

impl ToSchema for HostnamePort {}

impl TryInto<ockam_transport_core::HostnamePort> for HostnamePort {
    type Error = ockam_core::Error;
    fn try_into(self) -> Result<ockam_transport_core::HostnamePort, Self::Error> {
        ockam_transport_core::HostnamePort::new(self.hostname, self.port)
    }
}

impl TryFrom<&str> for HostnamePort {
    type Error = ockam_core::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let hostname = ockam_transport_core::HostnamePort::from_str(value)?;
        Ok(HostnamePort {
            hostname: hostname.hostname,
            port: hostname.port,
        })
    }
}

impl Serialize for HostnamePort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HostnamePort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HostnamePortVisitor;

        impl<'de> serde::de::Visitor<'de> for HostnamePortVisitor {
            type Value = HostnamePort;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a string in format 'hostname:port' or an object with 'hostname' and 'port' fields")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                HostnamePort::try_from(value).map_err(serde::de::Error::custom)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut hostname = None;
                let mut port = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "hostname" => {
                            if hostname.is_some() {
                                return Err(serde::de::Error::duplicate_field("hostname"));
                            }
                            hostname = Some(map.next_value::<String>()?);
                        }
                        "port" => {
                            if port.is_some() {
                                return Err(serde::de::Error::duplicate_field("port"));
                            }
                            port = Some(map.next_value::<u16>()?);
                        }
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                let hostname =
                    hostname.ok_or_else(|| serde::de::Error::missing_field("hostname"))?;
                let port = port.ok_or_else(|| serde::de::Error::missing_field("port"))?;

                Ok(HostnamePort { hostname, port })
            }
        }

        deserializer.deserialize_any(HostnamePortVisitor)
    }
}

impl Display for HostnamePort {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        ockam_transport_core::HostnamePort {
            hostname: self.hostname.clone(),
            port: self.port,
        }
        .fmt(f)
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectionStatus {
    Up,
    Down,
}

impl From<crate::ConnectionStatus> for ConnectionStatus {
    fn from(status: crate::ConnectionStatus) -> Self {
        match status {
            crate::ConnectionStatus::Up => ConnectionStatus::Up,
            crate::ConnectionStatus::Down => ConnectionStatus::Down,
        }
    }
}

pub fn default_authority() -> Authority {
    Authority::Project { name: None }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Authority {
    Project {
        /// Name of the project
        /// When omitted, the default project will be used
        name: Option<String>,
    },
    Provided {
        /// Multiaddress to the node that will be used as an authority;
        /// When omitted, the default node will be used
        #[schema(example = "/dnsaddr/my-authority.example.com/tcp/4001/secure/api")]
        route: String,
        /// Identifier of the authority node
        #[schema(example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a")]
        identity: String,
        // TODO: Add the possibility to specify the whole public identity
    },
}

pub fn default_project_name() -> String {
    "default".to_string()
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Project {
    Existing {
        /// Name of the project
        /// When omitted, the default project will be used
        name: Option<String>,
        /// Multiaddress to the node that will be used as an authority;
        /// The Multiaddress will only be embedded in the ticket, but not used
        /// for the actual connection;
        /// When omitted, the existing authority route will be used
        #[serde(default)]
        #[schema(example = "/dnsaddr/my-authority.example.com/tcp/4001/secure/api")]
        authority_route: Option<String>,
        /// Multiaddress to the node that will be used as a project;
        /// The Multiaddress will only be embedded in the ticket, but not used
        /// for the actual connection;
        /// When omitted, the existing project route will be used
        #[serde(default)]
        #[schema(example = "/dnsaddr/my-project.example.com/tcp/4000/service/api")]
        project_route: Option<String>,
    },
    Provided {
        /// Name of the project;
        /// When omitted, the default project will be used
        #[serde(default = "default_project_name")]
        #[schema(example = "my-project", default = default_project_name)]
        project_name: String,
        /// Multiaddress to the node that will be used as an authority;
        #[schema(example = "/dnsaddr/my-authority.example.com/tcp/4001/secure/api")]
        authority_route: String,
        /// Full public identity of the authority node
        #[schema(example = "81825837830101583285f...")]
        authority_change_history: String,
        /// Multiaddress to the node that will be used as a project;
        #[schema(example = "/dnsaddr/my-project.example.com/tcp/4000/service/api")]
        project_route: String,
        /// Full public identity of the project node
        #[schema(example = "81825837830101583285f...")]
        project_change_history: String,
    },
}

pub fn default_project_information() -> Project {
    Project::Existing {
        name: None,
        project_route: None,
        authority_route: None,
    }
}

impl Project {
    pub async fn to_project_authority(&self) -> ockam_core::Result<Authority> {
        match self {
            Project::Existing { name, .. } => Ok(Authority::Project { name: name.clone() }),
            Project::Provided {
                authority_route,
                authority_change_history,
                ..
            } => {
                let identity = Identity::import_from_string(
                    None,
                    authority_change_history,
                    Vault::create_verifying_vault(),
                )
                .await?;
                Ok(Authority::Provided {
                    route: authority_route.clone(),
                    identity: identity.identifier().to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_hostname_port_deserialize_string() {
        // Deserialize from string format
        let json = json!("localhost:8080");
        let result: HostnamePort = serde_json::from_value(json).unwrap();
        assert_eq!(result.hostname, "localhost");
        assert_eq!(result.port, 8080);

        // IPv4 address
        let json = json!("127.0.0.1:9000");
        let result: HostnamePort = serde_json::from_value(json).unwrap();
        assert_eq!(result.hostname, "127.0.0.1");
        assert_eq!(result.port, 9000);

        // IPv6 address
        let json = json!("[::1]:8080");
        let result: HostnamePort = serde_json::from_value(json).unwrap();
        assert_eq!(result.hostname, "[::1]");
        assert_eq!(result.port, 8080);

        // Just port
        let json = json!("8080");
        let result: HostnamePort = serde_json::from_value(json).unwrap();
        assert_eq!(result.hostname, "127.0.0.1"); // localhost
        assert_eq!(result.port, 8080);
    }

    #[test]
    fn test_hostname_port_deserialize_object() {
        // Deserialize from object format
        let json = json!({
            "hostname": "example.com",
            "port": 8080
        });
        let result: HostnamePort = serde_json::from_value(json).unwrap();
        assert_eq!(result.hostname, "example.com");
        assert_eq!(result.port, 8080);

        // Duplicate fields -- It keeps the last one
        let json = json!({
            "hostname": "example.com",
            "hostname": "duplicate.com",
            "port": 8080
        });
        let result = serde_json::from_value::<HostnamePort>(json).unwrap();
        assert_eq!(result.hostname, "duplicate.com");
        assert_eq!(result.port, 8080);
    }

    #[test]
    fn test_hostname_port_deserialize_error_cases() {
        // Invalid string format
        let json = json!("invalid_format");
        let result = serde_json::from_value::<HostnamePort>(json);
        assert!(result.is_err());

        // Missing hostname in object
        let json = json!({
            "port": 8080
        });
        let result = serde_json::from_value::<HostnamePort>(json);
        assert!(result.is_err());

        // Missing port in object
        let json = json!({
            "hostname": "example.com"
        });
        let result = serde_json::from_value::<HostnamePort>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_hostname_port_serde() {
        let original = HostnamePort {
            hostname: "example.com".to_string(),
            port: 8080,
        };

        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: HostnamePort = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original.hostname, deserialized.hostname);
        assert_eq!(original.port, deserialized.port);
    }
}
