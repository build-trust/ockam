use crate::{Lease, TTL};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LeaseProtocolPermissions {
    action: String,
    resource: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum LeaseProtocolOption {
    TTL(TTL),
    OrgId(String),
    Permissions(Vec<LeaseProtocolPermissions>),
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LeaseProtocolRequest {
    action: String,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    options: HashMap<String, LeaseProtocolOption>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    token_id: String,
}

impl LeaseProtocolRequest {
    pub fn new<A: ToString, T: ToString>(
        action: A,
        options: HashMap<String, LeaseProtocolOption>,
        token_id: T,
    ) -> Self {
        LeaseProtocolRequest {
            action: action.to_string(),
            options,
            token_id: token_id.to_string(),
        }
    }

    pub fn new_no_opts<A: ToString, T: ToString>(action: A, token_id: T) -> Self {
        Self::new(action, HashMap::new(), token_id)
    }

    pub fn from_json(json: &str) -> Option<Self> {
        match serde_json::from_str(json) {
            Ok(request) => Some(request),
            Err(_) => None,
        }
    }

    pub fn get<T: ToString>(token_id: T) -> Self {
        Self::new_no_opts("get", token_id)
    }

    pub fn revoke<T: ToString>(token_id: T) -> Self {
        Self::new_no_opts("revoke", token_id)
    }

    pub fn create<S: ToString, B: ToString>(ttl: TTL, org_id: S, bucket: B) -> Self {
        let mut options = HashMap::<String, LeaseProtocolOption>::new();

        options.insert("ttl".to_string(), LeaseProtocolOption::TTL(ttl));
        options.insert(
            "orgID".to_string(),
            LeaseProtocolOption::OrgId(org_id.to_string()),
        );

        let auth_type: HashMap<String, String> =
            [("type".to_string(), "authorizations".to_string())]
                .iter()
                .cloned()
                .collect();

        let read_auth_perm = LeaseProtocolPermissions {
            action: "read".to_string(),
            resource: auth_type,
        };

        let ockam_bucket_type = [
            ("type".to_string(), "buckets".to_string()),
            ("name".to_string(), bucket.to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        let ockam_bucket_write_perm = LeaseProtocolPermissions {
            action: "write".to_string(),
            resource: ockam_bucket_type,
        };

        options.insert(
            "permissions".to_string(),
            LeaseProtocolOption::Permissions(vec![read_auth_perm, ockam_bucket_write_perm]),
        );
        LeaseProtocolRequest::new("create", options, "")
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct LeaseProtocolResponse {
    result: String,
    #[serde(default)]
    lease: Lease,
}

impl LeaseProtocolResponse {
    pub fn lease(&self) -> Lease {
        self.lease.clone()
    }

    pub fn from_json(json: &str) -> Self {
        match serde_json::from_str(json) {
            Ok(response) => response,
            Err(e) => {
                error!("Error deserializing Lease response: {}", e);
                Self::failure()
            }
        }
    }

    pub fn failure() -> Self {
        LeaseProtocolResponse {
            result: "failure".to_string(),
            lease: Lease::default(),
        }
    }

    pub fn success(lease: Lease) -> Self {
        LeaseProtocolResponse {
            result: "success".to_string(),
            lease,
        }
    }

    pub fn is_success(&self) -> bool {
        self.result == "success"
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::lease::json_proto::{LeaseProtocolRequest, LeaseProtocolResponse};
    use crate::Lease;
    use std::time::SystemTime;

    #[test]
    fn test_serialize_get() {
        let req = LeaseProtocolRequest::get("123");
        let json = req.as_json();
        let after: LeaseProtocolRequest = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(after, req)
    }

    #[test]
    fn test_serialize_revoke() {
        let req = LeaseProtocolRequest::revoke("456");
        let json = serde_json::to_string(&req).unwrap();
        let after: LeaseProtocolRequest = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(after, req)
    }

    #[test]
    fn test_create() {
        let req = LeaseProtocolRequest::create(1000, "789", "bucket");
        let json = serde_json::to_string(&req).unwrap();
        let after: LeaseProtocolRequest = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(after, req)
    }

    #[test]
    fn test_response_failure() {
        let response = LeaseProtocolResponse::failure();
        let json = serde_json::to_string(&response).unwrap();
        let after: LeaseProtocolResponse = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(after, response);

        let response = LeaseProtocolResponse::failure();
        let json = serde_json::to_string(&response).unwrap();
        let after: LeaseProtocolResponse = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(after, response);

        let after: LeaseProtocolResponse = serde_json::from_str(r#"{"result":"failure"}"#).unwrap();
        assert!(!after.is_success())
    }

    #[test]
    fn test_response_success() {
        let now = || {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as usize
        };

        let lease = Lease::new("testing", 10_000);

        let response = LeaseProtocolResponse::success(lease);
        let json = serde_json::to_string(&response).unwrap();
        let after = LeaseProtocolResponse::from_json(json.as_str());
        assert!(after.is_success());

        let lease = after.lease;
        assert!(lease.is_valid(now()));
        assert_eq!("testing", lease.value());
        assert_eq!(10_000, lease.ttl());
        println!("{}", json);
    }
}
