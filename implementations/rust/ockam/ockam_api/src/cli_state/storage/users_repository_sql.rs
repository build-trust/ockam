use itertools::Itertools;
use sqlx::*;
use std::sync::Arc;

use crate::cloud::email_address::EmailAddress;
use crate::cloud::enroll::auth0::UserInfo;
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{Boolean, FromSqlxError, SqlxDatabase, ToVoid};

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

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn UsersRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("users").await?))
    }

    async fn set_as_default(
        &self,
        email: &EmailAddress,
        transaction: &mut AnyConnection,
    ) -> Result<()> {
        // set the user as the default one
        query(r#"UPDATE "user" SET is_default = $1 WHERE LOWER(email) = LOWER($2)"#)
            .bind(true)
            .bind(email)
            .execute(&mut *transaction)
            .await
            .void()?;

        // set all the others as non-default
        query(r#"UPDATE "user" SET is_default = $1 WHERE LOWER(email) <> LOWER($2)"#)
            .bind(false)
            .bind(email)
            .execute(&mut *transaction)
            .await
            .void()?;

        Ok(())
    }
}

#[async_trait]
impl UsersRepository for UsersSqlxDatabase {
    async fn store_user(&self, user: &UserInfo) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        // Set to default if there is no default user
        let is_default = {
            let default_user_email: Option<String> =
                query(r#"SELECT email FROM "user" WHERE is_default = $1"#)
                    .bind(true)
                    .fetch_optional(&mut *transaction)
                    .await
                    .into_core()?
                    .map(|row| row.get(0));
            let default_user_email: Option<EmailAddress> =
                default_user_email.map(|e| EmailAddress::new_unsafe(&e));
            default_user_email.is_none() || default_user_email.as_ref() == Some(&user.email)
        };

        // Get user if it exists, using the lowercased email
        let query1 =
            query(r#"SELECT email FROM "user" WHERE LOWER(email) = LOWER($1)"#).bind(&user.email);
        let existing_user: Option<String> = query1
            .fetch_optional(&mut *transaction)
            .await
            .into_core()?
            .map(|row| row.get(0));
        let email = existing_user.unwrap_or(user.email.to_string());

        let query2 = query(r#"
            INSERT INTO "user" (email, sub, nickname, name, picture, updated_at, email_verified, is_default)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (email)
            DO UPDATE SET sub = $2, nickname = $3, name = $4, picture = $5, updated_at = $6, email_verified = $7, is_default = $8"#)
            .bind(&email)
            .bind(&user.sub)
            .bind(&user.nickname)
            .bind(&user.name)
            .bind(&user.picture)
            .bind(&user.updated_at)
            .bind(user.email_verified)
            .bind(is_default);
        query2.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()
    }

    async fn get_default_user(&self) -> Result<Option<UserInfo>> {
        let query = query_as(r#"SELECT email, sub, nickname, name, picture, updated_at, email_verified, is_default FROM "user" WHERE is_default = $1"#).bind(true);
        let row: Option<UserRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|u| u.user()).transpose()?)
    }

    async fn set_default_user(&self, email: &EmailAddress) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        self.set_as_default(email, &mut transaction).await?;
        transaction.commit().await.void()
    }

    async fn get_user(&self, email: &EmailAddress) -> Result<Option<UserInfo>> {
        let query = query_as(
            r#"SELECT email, sub, nickname, name,
        picture, updated_at, email_verified, is_default
        FROM "user"
        WHERE LOWER(email) = LOWER($1)"#,
        )
        .bind(email);
        let row: Option<UserRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|u| u.user()).transpose()?)
    }

    async fn get_users(&self) -> Result<Vec<UserInfo>> {
        let query = query_as(
            r#"SELECT email, sub, nickname, name, picture, updated_at, email_verified, is_default FROM "user""#,
        );
        let rows: Vec<UserRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        rows.iter().map(|u| u.user()).collect()
    }

    async fn delete_user(&self, email: &EmailAddress) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        // Check if the space is the default one
        let q = query_scalar(
            r#"SELECT EXISTS(SELECT 1 FROM "user" WHERE LOWER(email) = LOWER($1) AND is_default = $2)"#,
        )
        .bind(email.to_string())
        .bind(true);
        let is_default: Boolean = q.fetch_one(&mut *transaction).await.into_core()?;
        let is_default = is_default.to_bool();

        let query1 = query(r#"DELETE FROM "user" WHERE LOWER(email) = LOWER($1)"#).bind(email);
        query1.execute(&mut *transaction).await.void()?;

        // Set another space as default if the deleted one was the default
        if is_default {
            let user_emails: Vec<String> = query_scalar(r#"SELECT email FROM "user""#)
                .fetch_all(&mut *transaction)
                .await
                .into_core()?;
            let user_emails: Vec<_> = user_emails
                .into_iter()
                .map(|e| EmailAddress::new_unsafe(&e))
                .unique()
                .collect();
            for email in user_emails {
                if self.set_as_default(&email, &mut transaction).await.is_ok() {
                    break;
                }
            }
        }

        transaction.commit().await.void()
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
    email_verified: Boolean,
    #[allow(unused)]
    is_default: Boolean,
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
            email_verified: self.email_verified.to_bool(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use ockam_node::database::with_sqlite_dbs;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        with_sqlite_dbs(|db| async move {
            let repository: Arc<dyn UsersRepository> = Arc::new(UsersSqlxDatabase::new(db));

            let my_email_address: EmailAddress = "me@ockam.io".try_into().unwrap();
            let your_email_address: EmailAddress = "you@ockam.io".try_into().unwrap();
            let your_email_address_capitalized: EmailAddress = "You@ockam.io".try_into().unwrap();

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
                sub: "ub".into(),
                nickname: "You".to_string(),
                name: "You".to_string(),
                picture: "You".to_string(),
                updated_at: "today".to_string(),
                email: your_email_address_capitalized.clone(),
                email_verified: false,
            };
            let user3 = UserInfo {
                sub: "sub".into(),
                nickname: "you".to_string(),
                name: "you".to_string(),
                picture: "you".to_string(),
                updated_at: "today".to_string(),
                email: your_email_address.clone(),
                email_verified: false,
            };

            repository.store_user(&user1).await?;
            // The first stored user is the default one
            let result = repository.get_default_user().await?;
            assert_eq!(result, Some(user1.clone()));

            repository.store_user(&user2).await?;
            // The second stored space is not the default one
            let result = repository.get_default_user().await?;
            assert_eq!(result, Some(user1.clone()));

            repository.store_user(&user3).await?;
            // The third space replaces the second one, as the email is equivalent
            let result = repository.get_user(&your_email_address).await?;
            assert_eq!(result, Some(user3.clone()));
            let result = repository.get_user(&your_email_address_capitalized).await?;
            assert_eq!(result, Some(user3.clone()));

            // retrieve them as a vector or by name
            let result = repository.get_users().await?;
            assert_eq!(result, vec![user1.clone(), user3.clone()]);

            let result = repository.get_user(&my_email_address).await?;
            assert_eq!(result, Some(user1.clone()));

            // a user can be set as the default user
            repository.set_default_user(&your_email_address).await?;
            let result = repository.get_default_user().await?;
            assert_eq!(result, Some(user3.clone()));

            // a user can be deleted
            repository.delete_user(&your_email_address).await?;
            let result = repository.get_user(&your_email_address).await?;
            assert_eq!(result, None);

            // if the default user is deleted, the next one becomes the default
            let result = repository.get_default_user().await?;
            assert_eq!(result, Some(user1.clone()));

            let result = repository.get_users().await?;
            assert_eq!(result, vec![user1.clone()]);
            Ok(())
        })
        .await
    }
}
