use crate::authenticator::direct::{OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY};
use crate::authenticator::AuthorityMembersRepository;
use ockam::identity::Identifier;
use ockam_core::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) struct EnrollerCheckResult {
    pub(crate) is_member: bool,
    pub(crate) is_enroller: bool,
    pub(crate) is_admin: bool,
    pub(crate) is_pre_trusted: bool,
}

pub(crate) struct EnrollerAccessControlChecks;

impl EnrollerAccessControlChecks {
    pub(crate) fn check_str_attributes_is_enroller(attributes: &BTreeMap<String, String>) -> bool {
        if let Some(val) = attributes.get(OCKAM_ROLE_ATTRIBUTE_KEY) {
            if val == OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE {
                return true;
            }
        }

        false
    }

    pub(crate) fn check_bin_attributes_is_enroller(
        attributes: &BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> bool {
        if let Some(val) = attributes.get(OCKAM_ROLE_ATTRIBUTE_KEY.as_bytes()) {
            if val == OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.as_bytes() {
                return true;
            }
        }

        false
    }

    pub(crate) async fn check_identifier(
        members: Arc<dyn AuthorityMembersRepository>,
        identifier: &Identifier,
    ) -> Result<EnrollerCheckResult> {
        match members.get_member(identifier).await? {
            Some(member) => {
                let is_enroller = Self::check_bin_attributes_is_enroller(member.attributes());
                Ok(EnrollerCheckResult {
                    is_member: true,
                    is_enroller,
                    is_admin: is_enroller, //TODO: use project admin credentials
                    is_pre_trusted: member.is_pre_trusted(),
                })
            }
            None => Ok(EnrollerCheckResult {
                is_member: false,
                is_enroller: false,
                is_admin: false,
                is_pre_trusted: false,
            }),
        }
    }
}
