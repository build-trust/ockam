use serde::{Deserialize, Serialize};

use ockam_api::cloud::enroll::auth0::UserInfo;
use ockam_api::nodes::models::portal::OutletStatus;

/// The ModelState stores all the data which is not maintained by the NodeManager:
///    - user information
///    - shared services
///    - sent invitations
///    - etc...
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelState {
    user_info: Option<UserInfo>,
    #[serde(default = "Vec::new")]
    pub(crate) tcp_outlets: Vec<OutletStatus>,
}

impl Default for ModelState {
    fn default() -> Self {
        ModelState::new(None, vec![])
    }
}

impl ModelState {
    pub fn new(user_info: Option<UserInfo>, tcp_outlets: Vec<OutletStatus>) -> Self {
        Self {
            user_info,
            tcp_outlets,
        }
    }

    pub fn set_user_info(&mut self, user_info: UserInfo) {
        self.user_info = Some(user_info)
    }

    pub fn get_user_info(&self) -> Option<UserInfo> {
        self.user_info.clone()
    }
}
