use sqlx::*;

use crate::cloud::email_address::EmailAddress;
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};

use crate::cloud::enroll::auth0::UserInfo;

use super::UsersRepository;

#[derive(Clone)]
pub struct UsersSqlxDatabase {
    database: SqlxDatabase,
}

impl UsersSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for users");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("users").await?))
    }
}

#[async_trait]
impl UsersRepository for UsersSqlxDatabase {
    async fn store_user(&self, user: &UserInfo) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query1 =
            query_scalar("SELECT EXISTS(SELECT email FROM user WHERE is_default=$1 AND email=$2)")
                .bind(true.to_sql())
                .bind(user.email.to_sql());
        let is_already_default: bool = query1.fetch_one(&mut *transaction).await.into_core()?;

        let query2 = query("INSERT OR REPLACE INTO user VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(user.email.to_sql())
            .bind(user.sub.to_sql())
            .bind(user.nickname.to_sql())
            .bind(user.name.to_sql())
            .bind(user.picture.to_sql())
            .bind(user.updated_at.to_sql())
            .bind(user.email_verified.to_sql())
            .bind(is_already_default.to_sql());
        query2.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()
    }

    async fn get_default_user(&self) -> Result<Option<UserInfo>> {
        let query = query_as("SELECT email, sub, nickname, name, picture, updated_at, email_verified, is_default FROM user WHERE is_default=$1").bind(true.to_sql());
        let row: Option<UserRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|u| u.user()).transpose()?)
    }

    async fn set_default_user(&self, email: &EmailAddress) -> Result<()> {
        let query = query("UPDATE user SET is_default = ? WHERE email = ?")
            .bind(true.to_sql())
            .bind(email.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_user(&self, email: &EmailAddress) -> Result<Option<UserInfo>> {
        let query = query_as("SELECT email, sub, nickname, name, picture, updated_at, email_verified, is_default FROM user WHERE email=$1").bind(email.to_sql());
        let row: Option<UserRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|u| u.user()).transpose()?)
    }

    async fn get_users(&self) -> Result<Vec<UserInfo>> {
        let query = query_as("SELECT email, sub, nickname, name, picture, updated_at, email_verified, is_default FROM user");
        let rows: Vec<UserRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        rows.iter().map(|u| u.user()).collect()
    }

    async fn delete_user(&self, email: &EmailAddress) -> Result<()> {
        let query1 = query("DELETE FROM user WHERE email=?").bind(email.to_sql());
        query1.execute(&*self.database.pool).await.void()
    }
}

// Database serialization / deserialization

/// Low-level representation of a row in the user table
#[derive(sqlx::FromRow)]
struct UserRow {
    email: String,
    sub: String,
    nickname: String,
    name: String,
    picture: String,
    updated_at: String,
    email_verified: bool,
    #[allow(unused)]
    is_default: bool,
}

impl UserRow {
    fn user(&self) -> Result<UserInfo> {
        Ok(UserInfo {
            email: self.email.clone().try_into()?,
            sub: self.sub.clone(),
            nickname: self.nickname.clone(),
            name: self.name.clone(),
            picture: self.picture.clone(),
            updated_at: self.updated_at.clone(),
            email_verified: self.email_verified,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;
        let my_email_address: EmailAddress = "me@ockam.io".try_into().unwrap();
        let your_email_address: EmailAddress = "you@ockam.io".try_into().unwrap();

        // create and store 2 users
        let user1 = UserInfo {
            sub: "sub".into(),
            nickname: "me".to_string(),
            name: "me".to_string(),
            picture: "me".to_string(),
            updated_at: "today".to_string(),
            email: my_email_address.clone(),
            email_verified: false,
        };
        let user2 = UserInfo {
            sub: "sub".into(),
            nickname: "you".to_string(),
            name: "you".to_string(),
            picture: "you".to_string(),
            updated_at: "today".to_string(),
            email: your_email_address.clone(),
            email_verified: false,
        };

        repository.store_user(&user1).await?;
        repository.store_user(&user2).await?;

        // retrieve them as a vector or by name
        let result = repository.get_users().await?;
        assert_eq!(result, vec![user1.clone(), user2.clone()]);

        let result = repository.get_user(&my_email_address).await?;
        assert_eq!(result, Some(user1.clone()));

        // a user can be set created as the default user
        repository.set_default_user(&my_email_address).await?;
        let result = repository.get_default_user().await?;
        assert_eq!(result, Some(user1.clone()));

        // a user can be deleted
        repository.delete_user(&your_email_address).await?;
        let result = repository.get_user(&your_email_address).await?;
        assert_eq!(result, None);

        let result = repository.get_users().await?;
        assert_eq!(result, vec![user1.clone()]);
        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn UsersRepository>> {
        Ok(Arc::new(UsersSqlxDatabase::create().await?))
    }
}
