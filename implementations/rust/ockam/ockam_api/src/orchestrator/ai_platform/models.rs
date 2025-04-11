use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub zone: String,
    pub customer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcrCredentials {
    pub customer: String,
    pub image_name: String,
    pub repository_uri: String,
    pub auth_token: String,
}
