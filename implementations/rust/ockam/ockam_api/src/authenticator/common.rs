use crate::authenticator::direct::{OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY};
use crate::authenticator::AuthorityMembersRepository;
use ockam::identity::{Identifier, IdentitiesAttributes};
use ockam_core::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::direct::AccountAuthorityInfo;

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

    pub(crate) async fn check_is_member(
        authority: &Identifier,
        members: Arc<dyn AuthorityMembersRepository>,
        identifier: &Identifier,
    ) -> Result<EnrollerCheckResult> {
        let r = match members.get_member(authority, identifier).await? {
            Some(member) => {
                let is_enroller = Self::check_bin_attributes_is_enroller(member.attributes());
                EnrollerCheckResult {
                    is_member: true,
                    is_enroller,
                    is_admin: false,
                    is_pre_trusted: member.is_pre_trusted(),
                }
            }
            None => EnrollerCheckResult {
                is_member: false,
                is_enroller: false,
                is_admin: false,
                is_pre_trusted: false,
            },
        };

        Ok(r)
    }

    pub(crate) async fn check_identifier(
        authority: &Identifier,
        members: Arc<dyn AuthorityMembersRepository>,
        identities_attributes: Arc<IdentitiesAttributes>,
        identifier: &Identifier,
        account_authority: &Option<AccountAuthorityInfo>,
    ) -> Result<EnrollerCheckResult> {
        let mut r = Self::check_is_member(authority, members, identifier).await?;

        if let Some(info) = account_authority {
            if let Some(attrs) = identities_attributes
                .get_attributes(identifier, info.account_authority())
                .await?
            {
                if attrs.attrs().get("project".as_bytes())
                    == Some(&info.project_identifier().as_bytes().to_vec())
                {
                    r.is_admin = true;
                    r.is_enroller = true;
                    r.is_member = true;
                }
            }
            r.is_admin = r.is_admin || (!info.enforce_admin_checks() && r.is_enroller);
        } else {
            // If there is no account authority configured, treat enrollers as admins.
            // To be removed when we stop supporting legacy clients.
            r.is_admin = r.is_enroller;
        }
        Ok(r)
    }
}
