use core::ops::Deref;
use sqlx::*;
use std::sync::Arc;
use tracing::debug;

use crate::authenticator::{
    AuthorityMember, AuthorityMemberRow, AuthorityMembersRepository, PreTrustedIdentities,
};
use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};

#[derive(Clone)]
pub struct AuthorityMembersSqlxDatabase {
    database: SqlxDatabase,
}

impl AuthorityMembersSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for authority members");
        Self { database }
    }

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn AuthorityMembersRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("authority members").await?,
        ))
    }
}

#[async_trait]
impl AuthorityMembersRepository for AuthorityMembersSqlxDatabase {
    async fn get_member(
        &self,
        authority: &Identifier,
        identifier: &Identifier,
    ) -> Result<Option<AuthorityMember>> {
        let query = query_as("SELECT identifier, added_by, added_at, is_pre_trusted, attributes FROM authority_member WHERE authority_id = $1 AND identifier = $2")
            .bind(authority)
            .bind(identifier)
            ;
        let row: Option<AuthorityMemberRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.try_into()).transpose()
    }

    async fn get_members(&self, authority: &Identifier) -> Result<Vec<AuthorityMember>> {
        let query = query_as("SELECT identifier, added_by, added_at, is_pre_trusted, attributes FROM authority_member WHERE authority_id = $1");
        let row: Vec<AuthorityMemberRow> = query
            .bind(authority)
            .fetch_all(&*self.database.pool)
            .await
            .into_core()?;
        row.into_iter().map(|r| r.try_into()).collect()
    }

    async fn delete_member(&self, authority: &Identifier, identifier: &Identifier) -> Result<()> {
        let query =
            query("DELETE FROM authority_member WHERE authority_id = $1 AND identifier = $2 AND is_pre_trusted = $3")
                .bind(authority)
                .bind(identifier)
                .bind(false);
        query.execute(&*self.database.pool).await.void()
    }

    async fn add_member(&self, authority: &Identifier, member: AuthorityMember) -> Result<()> {
        let query = query(r#"
             INSERT INTO authority_member (identifier, added_by, added_at, is_pre_trusted, attributes, authority_id)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (identifier)
             DO UPDATE SET added_by = $2, added_at = $3, is_pre_trusted = $4, attributes = $5, authority_id = $6"#)
            .bind(member.identifier())
            .bind(member.added_by())
            .bind(member.added_at())
            .bind(member.is_pre_trusted())
            .bind(ockam_core::cbor_encode_preallocate(member.attributes())?)
            .bind(authority);

        query.execute(&*self.database.pool).await.void()
    }

    async fn bootstrap_pre_trusted_members(
        &self,
        authority: &Identifier,
        pre_trusted_identities: &PreTrustedIdentities,
    ) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 =
            query("DELETE FROM authority_member WHERE authority_id = $1 AND is_pre_trusted = $2")
                .bind(authority)
                .bind(true);
        query1.execute(&mut *transaction).await.void()?;

        for (identifier, pre_trusted_identity) in pre_trusted_identities.deref() {
            let query2 =
                query(r#"
                      INSERT INTO authority_member (identifier, added_by, added_at, is_pre_trusted, attributes, authority_id)
                      VALUES ($1, $2, $3, $4, $5, $6)
                      ON CONFLICT (identifier)
                      DO UPDATE SET added_by = $2, added_at = $3, is_pre_trusted = $4, attributes = $5, authority_id = $6"#)
                    .bind(identifier)
                    .bind(pre_trusted_identity.attested_by())
                    .bind(pre_trusted_identity.added_at())
                    .bind(true)
                    .bind(ockam_core::cbor_encode_preallocate(pre_trusted_identity.attrs())?)
                    .bind(authority);

            query2.execute(&mut *transaction).await.void()?;
        }

        transaction.commit().await.void()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authenticator::direct::OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE;
    use crate::authenticator::PreTrustedIdentity;
    use ockam::identity::models::IDENTIFIER_LEN;
    use ockam::identity::utils::now;
    use ockam::identity::Identifier;
    use ockam_core::compat::collections::BTreeMap;
    use ockam_core::compat::rand::RngCore;
    use ockam_core::compat::sync::Arc;
    use ockam_node::database::with_dbs;
    use rand::thread_rng;

    fn random_identifier() -> Identifier {
        let mut data = [0u8; IDENTIFIER_LEN];

        let mut rng = thread_rng();
        rng.fill_bytes(&mut data);

        Identifier(data)
    }

    #[tokio::test]
    async fn test_authority_members_repository_crud() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn AuthorityMembersRepository> =
                Arc::new(AuthorityMembersSqlxDatabase::new(db));

            let admin = random_identifier();
            let timestamp1 = now()?;

            let authority = random_identifier();
            let identifier1 = random_identifier();
            let mut attributes1 = BTreeMap::<Vec<u8>, Vec<u8>>::default();
            attributes1.insert(
                "role".as_bytes().to_vec(),
                OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.as_bytes().to_vec(),
            );
            let member1 = AuthorityMember::new(
                identifier1.clone(),
                attributes1,
                admin.clone(),
                timestamp1,
                false,
            );
            repository.add_member(&authority, member1.clone()).await?;

            let m1 = repository.get_member(&authority, &identifier1).await?;
            assert_eq!(m1, Some(member1.clone()));

            let members = repository.get_members(&authority).await?;
            assert_eq!(members.len(), 1);
            assert!(members.contains(&member1));

            let identifier2 = random_identifier();
            let mut attributes2 = BTreeMap::<Vec<u8>, Vec<u8>>::default();
            attributes2.insert("role".as_bytes().to_vec(), "user".as_bytes().to_vec());
            let timestamp2 = timestamp1 + 10;
            let member2 = AuthorityMember::new(
                identifier2.clone(),
                attributes2,
                admin.clone(),
                timestamp2,
                false,
            );
            repository.add_member(&authority, member2.clone()).await?;

            let members = repository.get_members(&authority).await?;
            assert_eq!(members.len(), 2);
            assert!(members.contains(&member1));
            assert!(members.contains(&member2));

            repository.delete_member(&authority, &identifier1).await?;

            let members = repository.get_members(&authority).await?;
            assert_eq!(members.len(), 1);
            assert!(members.contains(&member2));

            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_authority_members_repository_bootstrap() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn AuthorityMembersRepository> =
                Arc::new(AuthorityMembersSqlxDatabase::new(db));

            let mut pre_trusted_identities = BTreeMap::<Identifier, PreTrustedIdentity>::default();

            let timestamp1 = now()?;

            let authority = random_identifier();
            let identifier1 = random_identifier();
            let mut attributes1 = BTreeMap::<Vec<u8>, Vec<u8>>::default();
            attributes1.insert(
                "role".as_bytes().to_vec(),
                OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.as_bytes().to_vec(),
            );

            pre_trusted_identities.insert(
                identifier1.clone(),
                PreTrustedIdentity::new(attributes1.clone(), timestamp1, None, authority.clone()),
            );

            let identifier2 = random_identifier();
            let mut attributes2 = BTreeMap::<Vec<u8>, Vec<u8>>::default();
            attributes2.insert("role".as_bytes().to_vec(), "user".as_bytes().to_vec());
            let timestamp2 = timestamp1 + 10;
            let timestamp3 = timestamp2 + 10;

            pre_trusted_identities.insert(
                identifier2.clone(),
                PreTrustedIdentity::new(
                    attributes2.clone(),
                    timestamp2,
                    Some(timestamp3),
                    identifier1.clone(),
                ),
            );

            repository
                .bootstrap_pre_trusted_members(&authority, &pre_trusted_identities.into())
                .await?;

            let members = repository.get_members(&authority).await?;
            assert_eq!(members.len(), 2);
            let member1 = members
                .iter()
                .find(|x| x.identifier() == &identifier1)
                .unwrap();
            assert_eq!(member1.added_at(), timestamp1);
            assert_eq!(member1.added_by(), &authority);
            assert_eq!(member1.attributes(), &attributes1);
            assert!(member1.is_pre_trusted());

            let member2 = members
                .iter()
                .find(|x| x.identifier() == &identifier2)
                .unwrap();
            assert_eq!(member2.added_at(), timestamp2);
            assert_eq!(member2.added_by(), &identifier1);
            assert_eq!(member2.attributes(), &attributes2);
            assert!(member2.is_pre_trusted());

            repository.delete_member(&authority, &identifier1).await?;

            let members = repository.get_members(&authority).await?;
            assert_eq!(members.len(), 2);
            assert!(members.contains(member2));
            assert!(members.contains(member1));

            Ok(())
        })
        .await
    }
}
