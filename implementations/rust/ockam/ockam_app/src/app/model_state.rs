use serde::{Deserialize, Serialize};

use ockam_api::cloud::enroll::auth0::UserInfo;

/// The ModelState stores all the data which is not maintained by the NodeManager:
///    - user information
///    - shared services
///    - sent invitations
///    - etc...
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelState {
    user_info: Option<UserInfo>,
}

impl Default for ModelState {
    fn default() -> Self {
        ModelState::new(None)
    }
}

impl ModelState {
    pub fn new(user_info: Option<UserInfo>) -> Self {
        Self { user_info }
    }

    pub fn set_user_info(&mut self, user_info: UserInfo) {
        self.user_info = Some(user_info)
    }

    pub fn get_user_info(&self) -> Option<UserInfo> {
        self.user_info.clone()
    }
}
