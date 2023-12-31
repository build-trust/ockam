use sqlx::sqlite::SqliteRow;
use sqlx::*;

use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};

use crate::cloud::space::Space;

use super::SpacesRepository;

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

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("spaces").await?))
    }
}

#[async_trait]
impl SpacesRepository for SpacesSqlxDatabase {
    async fn store_space(&self, space: &Space) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query1 =
            query_scalar("SELECT EXISTS (SELECT 1 FROM space WHERE is_default=$1 AND space_id=$2)")
                .bind(true.to_sql())
                .bind(space.id.to_sql());
        let is_already_default: bool = query1.fetch_one(&mut *transaction).await.into_core()?;

        let query2 = query("INSERT OR REPLACE INTO space VALUES (?, ?, ?)")
            .bind(space.id.to_sql())
            .bind(space.name.to_sql())
            .bind(is_already_default.to_sql());
        query2.execute(&mut *transaction).await.void()?;

        // remove any existing users related to that space if any
        let query3 = query("DELETE FROM user_space WHERE space_id=$1").bind(space.id.to_sql());
        query3.execute(&mut *transaction).await.void()?;

        // store the users associated to that space
        for user_email in &space.users {
            let query4 = query("INSERT OR REPLACE INTO user_space VALUES (?, ?)")
                .bind(user_email.to_sql())
                .bind(space.id.to_sql());
            query4.execute(&mut *transaction).await.void()?;
        }

        transaction.commit().await.void()
    }

    async fn get_space(&self, space_id: &str) -> Result<Option<Space>> {
        let query = query("SELECT space_name FROM space WHERE space_id=$1").bind(space_id.to_sql());
        let row: Option<SqliteRow> = query
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

        let query1 = query_as("SELECT space_id, space_name FROM space WHERE space_name=$1")
            .bind(name.to_sql());
        let row: Option<SpaceRow> = query1.fetch_optional(&mut *transaction).await.into_core()?;
        let space = match row.map(|r| r.space()) {
            Some(mut space) => {
                let query2 =
                    query_as("SELECT space_id, user_email FROM user_space WHERE space_id=$1")
                        .bind(space.id.to_sql());
                let rows: Vec<UserSpaceRow> =
                    query2.fetch_all(&mut *transaction).await.into_core()?;
                let users = rows.into_iter().map(|r| r.user_email).collect();
                space.users = users;
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
        let row: Vec<SpaceRow> = query.fetch_all(&mut *transaction).await.into_core()?;

        let mut spaces = vec![];
        for space_row in row {
            let query2 = query_as("SELECT space_id, user_email FROM user_space WHERE space_id=$1")
                .bind(space_row.space_id.to_sql());
            let rows: Vec<UserSpaceRow> = query2.fetch_all(&mut *transaction).await.into_core()?;
            let users = rows.into_iter().map(|r| r.user_email).collect();
            spaces.push(space_row.space_with_user_emails(users))
        }

        transaction.commit().await.void()?;

        Ok(spaces)
    }

    async fn get_default_space(&self) -> Result<Option<Space>> {
        let query = query("SELECT space_name FROM space WHERE is_default=$1").bind(true.to_sql());
        let row: Option<SqliteRow> = query
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
        // set the space as the default one
        let query1 = query("UPDATE space SET is_default = ? WHERE space_id = ?")
            .bind(true.to_sql())
            .bind(space_id.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE space SET is_default = ? WHERE space_id <> ?")
            .bind(false.to_sql())
            .bind(space_id.to_sql());
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }

    async fn delete_space(&self, space_id: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query1 = query("DELETE FROM space WHERE space_id=?").bind(space_id.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        let query2 = query("DELETE FROM user_space WHERE space_id=?").bind(space_id.to_sql());
        query2.execute(&mut *transaction).await.void()?;

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
        self.space_with_user_emails(vec![])
    }

    pub(crate) fn space_with_user_emails(&self, user_emails: Vec<String>) -> Space {
        Space {
            id: self.space_id.clone(),
            name: self.space_name.clone(),
            users: user_emails,
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

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // create and store 2 spaces
        let space1 = Space {
            id: "1".to_string(),
            name: "name1".to_string(),
            users: vec!["me@ockam.io".to_string(), "you@ockam.io".to_string()],
        };
        let mut space2 = Space {
            id: "2".to_string(),
            name: "name2".to_string(),
            users: vec![
                "me@ockam.io".to_string(),
                "him@ockam.io".to_string(),
                "her@ockam.io".to_string(),
            ],
        };

        repository.store_space(&space1).await?;
        repository.store_space(&space2).await?;

        // retrieve them as a vector or by name
        let result = repository.get_spaces().await?;
        assert_eq!(result, vec![space1.clone(), space2.clone()]);

        let result = repository.get_space_by_name("name1").await?;
        assert_eq!(result, Some(space1.clone()));

        // a space can be marked as the default space
        repository.set_default_space("1").await?;
        let result = repository.get_default_space().await?;
        assert_eq!(result, Some(space1.clone()));

        repository.set_default_space("2").await?;
        let result = repository.get_default_space().await?;
        assert_eq!(result, Some(space2.clone()));

        // updating a space which was already the default should keep it the default
        space2.users = vec!["someone@ockam.io".to_string()];
        repository.store_space(&space2).await?;
        let result = repository.get_default_space().await?;
        assert_eq!(result, Some(space2.clone()));

        // a space can be deleted
        repository.delete_space("2").await?;
        let result = repository.get_default_space().await?;
        assert_eq!(result, None);

        let result = repository.get_spaces().await?;
        assert_eq!(result, vec![space1.clone()]);
        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn SpacesRepository>> {
        Ok(Arc::new(SpacesSqlxDatabase::create().await?))
    }
}
