use super::SpacesRepository;
use crate::cloud::space::Space;
use crate::cloud::subscription::{Subscription, SubscriptionName};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{Boolean, FromSqlxError, Nullable, SqlxDatabase, ToVoid};
use sqlx::any::AnyRow;
use sqlx::*;
use std::str::FromStr;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct SpacesSqlxDatabase {
    database: SqlxDatabase,
}

impl SpacesSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for spaces");
        Self { database }
    }

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn SpacesRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("spaces").await?))
    }

    async fn query_subscription(&self, space_id: &str) -> Result<Option<Subscription>> {
        let query = query_as("SELECT space_id, name, is_free_trial, marketplace, start_date, end_date FROM subscription WHERE space_id = $1").bind(space_id);
        let row: Option<SubscriptionRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.subscription()).transpose()
    }

    async fn set_as_default(&self, space_id: &str, transaction: &mut AnyConnection) -> Result<()> {
        // set the space as the default one
        let query1 = query("UPDATE space SET is_default = $1 WHERE space_id = $2")
            .bind(true)
            .bind(space_id);
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE space SET is_default = $1 WHERE space_id <> $2")
            .bind(false)
            .bind(space_id);
        query2.execute(&mut *transaction).await.void()?;

        Ok(())
    }
}

#[async_trait]
impl SpacesRepository for SpacesSqlxDatabase {
    async fn store_space(&self, space: &Space) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        // Set to default if there is no default space
        let is_default = {
            let default_space_id: Option<String> =
                query("SELECT space_id FROM space WHERE is_default = $1")
                    .bind(true)
                    .fetch_optional(&mut *transaction)
                    .await
                    .into_core()?
                    .map(|row| row.get(0));
            default_space_id.is_none() || default_space_id.as_ref() == Some(&space.id)
        };

        let query2 = query(
            r#"
             INSERT INTO space (space_id, space_name, is_default)
             VALUES ($1, $2, $3)
             ON CONFLICT (space_id)
             DO UPDATE SET space_name = $2, is_default = $3"#,
        )
        .bind(&space.id)
        .bind(&space.name)
        .bind(is_default);
        query2.execute(&mut *transaction).await.void()?;

        if is_default {
            self.set_as_default(&space.id, &mut transaction).await?;
        }

        // remove any existing users related to that space if any
        let query3 = query("DELETE FROM user_space WHERE space_id = $1").bind(&space.id);
        query3.execute(&mut *transaction).await.void()?;

        // store the users associated to that space
        for user_email in &space.users {
            let query4 = query(
                r#"
              INSERT INTO user_space (user_email, space_id)
              VALUES ($1, $2)
              ON CONFLICT DO NOTHING"#,
            )
            .bind(user_email)
            .bind(&space.id);
            query4.execute(&mut *transaction).await.void()?;
        }

        // store the subscription if any
        if let Some(subscription) = &space.subscription {
            let start_date = subscription.start_date();
            let end_date = subscription.end_date();
            let query = query(
                r"
             INSERT INTO subscription (space_id, name, is_free_trial, marketplace, start_date, end_date)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (space_id)
             DO UPDATE SET space_id = $1, name = $2, is_free_trial = $3, marketplace = $4, start_date = $5, end_date = $6",
            )
                .bind(&space.id)
                .bind(subscription.name.to_string())
                .bind(subscription.is_free_trial)
                .bind(&subscription.marketplace)
                .bind(start_date.map(|d| d.into_inner().unix_timestamp()))
                .bind(end_date.map(|d| d.into_inner().unix_timestamp()));
            query.execute(&mut *transaction).await.void()?;
        }
        // remove the subscription
        else {
            let query = query("DELETE FROM subscription WHERE space_id = $1").bind(&space.id);
            query.execute(&mut *transaction).await.void()?;
        }

        transaction.commit().await.void()
    }

    async fn get_space(&self, space_id: &str) -> Result<Option<Space>> {
        let query = query("SELECT space_name FROM space WHERE space_id = $1").bind(space_id);
        let row: Option<AnyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        match row {
            Some(r) => {
                let space_name: String = r.get(0);
                self.get_space_by_name(&space_name).await
            }
            None => Ok(None),
        }
    }

    async fn get_space_by_name(&self, name: &str) -> Result<Option<Space>> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query1 =
            query_as("SELECT space_id, space_name FROM space WHERE space_name = $1").bind(name);
        let row: Option<SpaceRow> = query1.fetch_optional(&mut *transaction).await.into_core()?;
        let space = match row.map(|r| r.space()) {
            Some(mut space) => {
                // retrieve the users
                let query2 =
                    query_as("SELECT space_id, user_email FROM user_space WHERE space_id = $1")
                        .bind(&space.id);
                let rows: Vec<UserSpaceRow> =
                    query2.fetch_all(&mut *transaction).await.into_core()?;
                let users = rows.into_iter().map(|r| r.user_email).collect();
                space.users = users;

                // retrieve the subscription
                space.subscription = self.query_subscription(&space.id).await?;

                Some(space)
            }
            None => None,
        };
        transaction.commit().await.void()?;
        Ok(space)
    }

    async fn get_spaces(&self) -> Result<Vec<Space>> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query = query_as("SELECT space_id, space_name FROM space");
        let rows: Vec<SpaceRow> = query.fetch_all(&mut *transaction).await.into_core()?;

        let mut spaces = vec![];
        for row in rows {
            let query2 =
                query_as("SELECT space_id, user_email FROM user_space WHERE space_id = $1")
                    .bind(&row.space_id);
            let rows: Vec<UserSpaceRow> = query2.fetch_all(&mut *transaction).await.into_core()?;
            let users = rows.into_iter().map(|r| r.user_email).collect();
            let subscription = self.query_subscription(&row.space_id).await?;
            let mut space = row.space();
            space.users = users;
            space.subscription = subscription;
            spaces.push(space);
        }

        transaction.commit().await.void()?;

        Ok(spaces)
    }

    async fn get_default_space(&self) -> Result<Option<Space>> {
        let query = query("SELECT space_name FROM space WHERE is_default = $1").bind(true);
        let row: Option<AnyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        let name: Option<String> = row.map(|r| r.get(0));
        match name {
            Some(name) => self.get_space_by_name(&name).await,
            None => Ok(None),
        }
    }

    async fn set_default_space(&self, space_id: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        self.set_as_default(space_id, &mut transaction).await?;
        transaction.commit().await.void()
    }

    async fn delete_space(&self, space_id: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        // Check if the space is the default one
        let q = query_scalar(
            r#"SELECT EXISTS(SELECT 1 FROM space WHERE space_id = $1 AND is_default = $2)"#,
        )
        .bind(space_id.to_string())
        .bind(true);
        let is_default: Boolean = q.fetch_one(&mut *transaction).await.into_core()?;
        let is_default = is_default.to_bool();

        // Delete it
        let query1 = query("DELETE FROM space WHERE space_id = $1").bind(space_id);
        query1.execute(&mut *transaction).await.void()?;

        let query2 = query("DELETE FROM user_space WHERE space_id = $1").bind(space_id);
        query2.execute(&mut *transaction).await.void()?;

        let query3 = query("DELETE FROM subscription WHERE space_id = $1").bind(space_id);
        query3.execute(&mut *transaction).await.void()?;

        // Set another space as default if the deleted one was the default
        if is_default {
            let space_ids: Vec<String> = query_scalar("SELECT space_id FROM space")
                .fetch_all(&mut *transaction)
                .await
                .into_core()?;
            for space_id in space_ids {
                if self
                    .set_as_default(&space_id, &mut transaction)
                    .await
                    .is_ok()
                {
                    break;
                }
            }
        }

        transaction.commit().await.void()
    }
}

//  Database serialization / deserialization

/// Low-level representation of a row in the space table
#[derive(sqlx::FromRow)]
struct SpaceRow {
    space_id: String,
    space_name: String,
}

impl SpaceRow {
    pub(crate) fn space(&self) -> Space {
        Space {
            id: self.space_id.clone(),
            name: self.space_name.clone(),
            users: vec![],
            subscription: None,
        }
    }
}

/// Low-level representation of a row in the user_space table
#[derive(sqlx::FromRow)]
struct UserSpaceRow {
    #[allow(unused)]
    space_id: String,
    user_email: String,
}

/// Low-level representation of a row in the subscription table
#[derive(sqlx::FromRow)]
pub(super) struct SubscriptionRow {
    #[allow(unused)]
    space_id: String,
    name: String,
    is_free_trial: Boolean,
    marketplace: Nullable<String>,
    start_date: Nullable<i64>,
    end_date: Nullable<i64>,
}

impl SubscriptionRow {
    pub(crate) fn subscription(&self) -> Result<Subscription> {
        Ok(Subscription::new(
            SubscriptionName::from_str(&self.name)?,
            self.is_free_trial.to_bool(),
            self.marketplace.to_option(),
            self.start_date
                .to_option()
                .and_then(|t| OffsetDateTime::from_unix_timestamp(t).ok())
                .and_then(|t| t.try_into().ok()),
            self.end_date
                .to_option()
                .and_then(|t| OffsetDateTime::from_unix_timestamp(t).ok())
                .and_then(|t| t.try_into().ok()),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cloud::subscription::SubscriptionName;
    use ockam_node::database::with_sqlite_dbs;
    use std::ops::Add;
    use time::ext::NumericalDuration;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        with_sqlite_dbs(|db| async move {
            let repository = SpacesSqlxDatabase::new(db);

            // create and store 2 spaces
            let mut space1 = Space {
                id: "1".to_string(),
                name: "name1".to_string(),
                users: vec!["me@ockam.io".to_string(), "you@ockam.io".to_string()],
                subscription: None,
            };
            let space2 = Space {
                id: "2".to_string(),
                name: "name2".to_string(),
                users: vec![
                    "me@ockam.io".to_string(),
                    "him@ockam.io".to_string(),
                    "her@ockam.io".to_string(),
                ],
                subscription: Some(Subscription::new(
                    SubscriptionName::Basic,
                    false,
                    Some("aws".to_string()),
                    Some(OffsetDateTime::now_utc().try_into().unwrap()),
                    Some(OffsetDateTime::now_utc().add(2.days()).try_into().unwrap()),
                )),
            };

            repository.store_space(&space1).await?;
            // The first stored space is the default one
            let result = repository.get_default_space().await?;
            assert_eq!(result, Some(space1.clone()));

            repository.store_space(&space2).await?;
            // The second stored space is not the default one
            let result = repository.get_default_space().await?;
            assert_eq!(result, Some(space1.clone()));

            // subscription is stored
            let result = repository.query_subscription(&space1.id).await?;
            assert_eq!(result, None);
            let result = repository.query_subscription(&space2.id).await?;
            assert_eq!(result, Some(space2.subscription.clone().unwrap()));

            // retrieve them as a vector or by name
            let result = repository.get_spaces().await?;
            for space in vec![space1.clone(), space2.clone()] {
                assert!(result.contains(&space));
            }

            let result = repository.get_space_by_name("name1").await?;
            assert_eq!(result, Some(space1.clone()));

            // The second space can be marked as the default space
            repository.set_default_space(&space2.id).await?;
            let result = repository.get_default_space().await?;
            assert_eq!(result, Some(space2.clone()));

            // We can also revert to the first space as the default space
            repository.set_default_space(&space1.id).await?;
            let result = repository.get_default_space().await?;
            assert_eq!(result, Some(space1.clone()));

            // updating a space which was already the default should keep it the default
            space1.users = vec!["someone@ockam.io".to_string()];
            repository.store_space(&space1).await?;
            let result = repository.get_default_space().await?;
            assert_eq!(result, Some(space1.clone()));

            // a space can be deleted
            repository.delete_space(&space1.id).await?;
            let result = repository.get_default_space().await?;

            // if the default space is deleted, the next one becomes the default
            assert_eq!(result, Some(space2.clone()));

            let result = repository.get_spaces().await?;
            assert_eq!(result, vec![space2.clone()]);

            // subscription is deleted
            let result = repository.query_subscription(&space1.id).await?;
            assert_eq!(result, None);

            Ok(())
        })
        .await
    }
}
