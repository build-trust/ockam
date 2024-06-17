use core::ops::Deref;
use sqlx::*;
use tracing::debug;

use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};

use crate::authenticator::{
    AuthorityMember, AuthorityMemberRow, AuthorityMembersRepository, PreTrustedIdentities,
};

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

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("authority members").await?,
        ))
    }
}

#[async_trait]
impl AuthorityMembersRepository for AuthorityMembersSqlxDatabase {
    async fn get_member(&self, identifier: &Identifier) -> Result<Option<AuthorityMember>> {
        let query = query_as("SELECT identifier, attributes, added_by, added_at, is_pre_trusted FROM authority_member WHERE identifier=?")
            .bind(identifier.to_sql());
        let row: Option<AuthorityMemberRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.try_into()).transpose()
    }

    async fn get_members(&self) -> Result<Vec<AuthorityMember>> {
        let query = query_as("SELECT identifier, attributes, added_by, added_at, is_pre_trusted FROM authority_member");
        let row: Vec<AuthorityMemberRow> =
            query.fetch_all(&*self.database.pool).await.into_core()?;
        row.into_iter().map(|r| r.try_into()).collect()
    }

    async fn delete_member(&self, identifier: &Identifier) -> Result<()> {
        let query = query("DELETE FROM authority_member WHERE identifier=? AND is_pre_trusted=?")
            .bind(identifier.to_sql())
            .bind(false.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn add_member(&self, member: AuthorityMember) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO authority_member VALUES (?1, ?2, ?3, ?4, ?5)")
            .bind(member.identifier().to_sql())
            .bind(member.added_by().to_sql())
            .bind(member.added_at().to_sql())
            .bind(member.is_pre_trusted().to_sql())
            .bind(ockam_core::cbor_encode_preallocate(member.attributes())?.to_sql());

        query.execute(&*self.database.pool).await.void()
    }

    async fn bootstrap_pre_trusted_members(
        &self,
        pre_trusted_identities: &PreTrustedIdentities,
    ) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 =
            query("DELETE FROM authority_member WHERE is_pre_trusted=?").bind(true.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        for (identifier, pre_trusted_identity) in pre_trusted_identities.deref() {
            let query2 =
                query("INSERT OR REPLACE INTO authority_member VALUES (?1, ?2, ?3, ?4, ?5)")
                    .bind(identifier.to_sql())
                    .bind(pre_trusted_identity.attested_by().to_sql())
                    .bind(pre_trusted_identity.added_at().to_sql())
                    .bind(true.to_sql())
                    .bind(
                        ockam_core::cbor_encode_preallocate(pre_trusted_identity.attrs())?.to_sql(),
                    );

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
    use rand::thread_rng;

    fn random_identifier() -> Identifier {
        let mut data = [0u8; IDENTIFIER_LEN];

        let mut rng = thread_rng();
        rng.fill_bytes(&mut data);

        Identifier(data)
    }

    #[tokio::test]
    async fn test_authority_members_repository_crud() -> Result<()> {
        let repository = create_repository().await?;

        let admin = random_identifier();
        let timestamp1 = now()?;

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
        repository.add_member(member1.clone()).await?;

        let members = repository.get_members().await?;
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
        repository.add_member(member2.clone()).await?;

        let members = repository.get_members().await?;
        assert_eq!(members.len(), 2);
        assert!(members.contains(&member1));
        assert!(members.contains(&member2));

        repository.delete_member(&identifier1).await?;

        let members = repository.get_members().await?;
        assert_eq!(members.len(), 1);
        assert!(members.contains(&member2));

        Ok(())
    }

    #[tokio::test]
    async fn test_authority_members_repository_bootstrap() -> Result<()> {
        let repository = create_repository().await?;

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
            .bootstrap_pre_trusted_members(&pre_trusted_identities.into())
            .await?;

        let members = repository.get_members().await?;
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

        repository.delete_member(&identifier1).await?;

        let members = repository.get_members().await?;
        assert_eq!(members.len(), 2);
        assert!(members.contains(member2));
        assert!(members.contains(member1));

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn AuthorityMembersRepository>> {
        Ok(Arc::new(AuthorityMembersSqlxDatabase::create().await?))
    }
}
