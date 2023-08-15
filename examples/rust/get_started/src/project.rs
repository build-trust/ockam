use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::anyhow;
use serde_json::{Map, Value};

use ockam::identity::{Identifier, Identities, Identity};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_multiaddr::MultiAddr;

/// This struct contains the json data exported
/// when running `ockam project information > project.json`
pub struct Project {
    pub project_identifier: Identifier,
    pub authority_identity: Identity,
    pub authority_route: MultiAddr,
    pub project_route: MultiAddr,
}

/// Accessors for a Project
impl Project {
    /// Return the identity identifier of the project
    pub fn identifier(&self) -> Identifier {
        self.project_identifier.clone()
    }

    /// Return the identity of the authority
    pub fn authority_identity(&self) -> Identity {
        self.authority_identity.clone()
    }

    /// Return the identifier of the authority
    pub fn authority_identifier(&self) -> Identifier {
        self.authority_identity.identifier().clone()
    }

    /// Return the authority route
    pub fn authority_route(&self) -> MultiAddr {
        self.authority_route.clone()
    }

    /// Return the project route
    pub fn route(&self) -> MultiAddr {
        self.project_route.clone()
    }
}

/// Import a project identity into a Vault from a project.json path
/// and return a Project struct
pub async fn import_project(path: &str, identities: Arc<Identities>) -> Result<Project> {
    match read_json(path)? {
        Value::Object(values) => {
            let project_identifier = Identifier::from_str(get_field_as_str(&values, "identity")?.as_str())?;

            let authority_identity = get_field_as_str(&values, "authority_identity")?;
            let identities_creation = identities.identities_creation();
            let authority_public_identity = identities_creation
                .import(None, &hex::decode(authority_identity).unwrap())
                .await?;

            let authority_access_route = get_field_as_str(&values, "authority_access_route")?;
            let authority_route =
                MultiAddr::from_str(authority_access_route.as_str()).map_err(|_| error("incorrect multi address"))?;

            let project_access_route = get_field_as_str(&values, "access_route")?;
            let project_route =
                MultiAddr::from_str(project_access_route.as_str()).map_err(|_| error("incorrect multi address"))?;

            Ok(Project {
                project_identifier,
                authority_identity: authority_public_identity,
                authority_route,
                project_route,
            })
        }
        _ => Err(error("incorrect project format")),
    }
}

/// Read the contents of a file as JSON
fn read_json(path: &str) -> Result<Value> {
    let mut file = File::open(path).map_err(|_| error("Unable to open the file at {path}"))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let result: Value =
        serde_json::from_str(contents.as_ref()).map_err(|e| error(format!("incorrect json content: {e}").as_str()))?;
    Ok(result)
}

/// Return the value of a given key as a String (if the field name exists)
fn get_field_as_str(values: &Map<String, Value>, field_name: &str) -> Result<String> {
    (*values)
        .get(field_name)
        .and_then(|v| v.as_str())
        .ok_or_else(|| error(format!("missing field '{field_name}'").as_str()))
        .map(|s| s.to_owned())
}

/// Utility function to create an Error in this file
fn error(message: &str) -> Error {
    Error::new(Origin::Application, Kind::Invalid, anyhow!(message.to_string()))
}
