use either::Either;
use std::collections::{BTreeMap, HashMap};

use ockam::identity::utils::now;
use ockam::identity::Identifier;
use ockam::identity::{AttributesEntry, IdentitiesAttributes};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

use crate::authenticator::common::EnrollerAccessControlChecks;
use crate::authenticator::{AuthorityMember, AuthorityMembersRepository};

/// Identity attribute key that indicates the role of the subject
pub const OCKAM_ROLE_ATTRIBUTE_KEY: &str = "ockam-role";

/// Identity attribute value that indicates the enroller role of the subject
/// the corresponding key is [`OCKAM_ROLE_ATTRIBUTE_KEY`]
pub const OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE: &str = "enroller";

pub struct DirectAuthenticatorError(pub String);

pub type DirectAuthenticatorResult<T> = Either<T, DirectAuthenticatorError>;

pub struct DirectAuthenticator {
    members: Arc<dyn AuthorityMembersRepository>,
    identities_attributes: Arc<IdentitiesAttributes>,
    account_authority: Option<AccountAuthorityInfo>,
}
#[derive(Clone)]
pub struct AccountAuthorityInfo {
    account_authority: Identifier,
    project_identifier: String,
    enforce_admin_checks: bool,
}

impl AccountAuthorityInfo {
    pub fn new(
        account_authority: Identifier,
        project_identifier: String,
        enforce_admin_checks: bool,
    ) -> Self {
        Self {
            account_authority,
            project_identifier,
            enforce_admin_checks,
        }
    }

    pub fn account_authority(&self) -> &Identifier {
        &self.account_authority
    }
    pub fn project_identifier(&self) -> String {
        self.project_identifier.clone()
    }
    pub fn enforce_admin_checks(&self) -> bool {
        self.enforce_admin_checks
    }
}

impl DirectAuthenticator {
    pub fn new(
        members: Arc<dyn AuthorityMembersRepository>,
        identities_attributes: Arc<IdentitiesAttributes>,
        account_authority: Option<AccountAuthorityInfo>,
    ) -> Self {
        Self {
            members,
            identities_attributes,
            account_authority,
        }
    }

    #[instrument(skip_all, fields(enroller = %enroller, identifier = %identifier))]
    pub async fn add_member(
        &self,
        enroller: &Identifier,
        identifier: &Identifier,
        attributes: &BTreeMap<String, String>,
    ) -> Result<DirectAuthenticatorResult<()>> {
        let check = EnrollerAccessControlChecks::check_identifier(
            self.members.clone(),
            self.identities_attributes.clone(),
            enroller,
            &self.account_authority,
        )
        .await?;

        if !check.is_enroller {
            warn!(
                "{} is trying to add member {}, but {} is not an enroller",
                enroller, identifier, enroller
            );
            return Ok(Either::Right(DirectAuthenticatorError(
                "Non-enroller is trying to add a member".to_string(),
            )));
        }

        let attrs = attributes
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect();

        // Check if we're trying to create an enroller
        if EnrollerAccessControlChecks::check_bin_attributes_is_enroller(&attrs) {
            // Only pre-trusted identities will be able to add enrollers
            if !check.is_admin {
                warn!(
                    "Not pre trusted enroller {} is trying to create an enroller {}",
                    enroller, identifier
                );

                return Ok(Either::Right(DirectAuthenticatorError(
                    "Not pre trusted enroller is trying to create an enroller".to_string(),
                )));
            }
        }

        let member =
            AuthorityMember::new(identifier.clone(), attrs, enroller.clone(), now()?, false);

        if let Err(err) = self.members.add_member(member).await {
            warn!("Error adding member {} directly: {}", identifier, err);
            return Ok(Either::Right(DirectAuthenticatorError(
                "Error adding member".to_string(),
            )));
        }

        info!(
            "Successfully added a member {} by {}. Attributes: {:?}",
            identifier, enroller, attributes
        );

        Ok(Either::Left(()))
    }

    #[instrument(skip_all, fields(enroller = %enroller))]
    pub async fn show_member(
        &self,
        enroller: &Identifier,
        identifier: &Identifier,
    ) -> Result<DirectAuthenticatorResult<AttributesEntry>> {
        let check = EnrollerAccessControlChecks::check_identifier(
            self.members.clone(),
            self.identities_attributes.clone(),
            enroller,
            &self.account_authority,
        )
        .await?;

        if !check.is_enroller {
            warn!("Non-enroller {} is trying to retrieve a member", enroller);
            return Ok(Either::Right(DirectAuthenticatorError(
                "Non-enroller is trying to retrieve a member".to_string(),
            )));
        }

        match self.members.get_member(identifier).await? {
            Some(member) => {
                let entry = AttributesEntry::new(
                    member.attributes().clone(),
                    member.added_at(),
                    None,
                    Some(member.added_by().clone()),
                );
                Ok(Either::Left(entry))
            }
            None => {
                warn!("Member {} not found", identifier);
                Ok(Either::Right(DirectAuthenticatorError(format!(
                    "Member {} not found",
                    identifier
                ))))
            }
        }
    }

    #[instrument(skip_all, fields(enroller = %enroller))]
    pub async fn list_members(
        &self,
        enroller: &Identifier,
    ) -> Result<DirectAuthenticatorResult<HashMap<Identifier, AttributesEntry>>> {
        let check = EnrollerAccessControlChecks::check_identifier(
            self.members.clone(),
            self.identities_attributes.clone(),
            enroller,
            &self.account_authority,
        )
        .await?;

        if !check.is_enroller {
            warn!("Non-enroller {} is trying to list members", enroller);
            return Ok(Either::Right(DirectAuthenticatorError(
                "Non-enroller is trying to list members".to_string(),
            )));
        }

        let all_members = self.members.get_members().await?;

        let mut res = HashMap::<Identifier, AttributesEntry>::default();
        for member in all_members {
            let entry = AttributesEntry::new(
                member.attributes().clone(),
                member.added_at(),
                None,
                Some(member.added_by().clone()),
            );
            res.insert(member.identifier().clone(), entry);
        }

        Ok(Either::Left(res))
    }

    #[instrument(skip_all, fields(enroller = %enroller, identifier = %identifier))]
    pub async fn delete_member(
        &self,
        enroller: &Identifier,
        identifier: &Identifier,
    ) -> Result<DirectAuthenticatorResult<()>> {
        let check_enroller = EnrollerAccessControlChecks::check_identifier(
            self.members.clone(),
            self.identities_attributes.clone(),
            enroller,
            &self.account_authority,
        )
        .await?;

        if !check_enroller.is_enroller {
            warn!(
                "Non-enroller {} is trying to delete member {}",
                enroller, identifier
            );
            return Ok(Either::Right(DirectAuthenticatorError(
                "Non-enroller is trying to delete a member".to_string(),
            )));
        }

        let check_member = EnrollerAccessControlChecks::check_identifier(
            self.members.clone(),
            self.identities_attributes.clone(),
            identifier,
            &self.account_authority,
        )
        .await?;

        if check_member.is_pre_trusted {
            warn!(
                "Enroller {} is trying to delete a pre trusted identity {}",
                enroller, identifier
            );
            return Ok(Either::Right(DirectAuthenticatorError(
                "Enroller is trying to delete a pre trusted identity".to_string(),
            )));
        }

        if check_member.is_enroller && !check_enroller.is_admin {
            warn!(
                "Not admin {} is trying to delete enroller {}",
                enroller, identifier
            );
            return Ok(Either::Right(DirectAuthenticatorError(
                "Not admin is trying to delete an enroller".to_string(),
            )));
        }

        self.members.delete_member(identifier).await?;

        info!("Successfully deleted member {}", identifier);

        Ok(Either::Left(()))
    }
}
