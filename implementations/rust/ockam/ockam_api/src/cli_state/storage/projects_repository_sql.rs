use crate::cloud::addon::KafkaConfig;
use crate::cloud::email_address::EmailAddress;
use crate::cloud::project::models::{OktaConfig, ProjectModel, ProjectUserRole};
use crate::cloud::share::{RoleInShare, ShareScope};
use crate::minicbor_url::Url;
use itertools::Itertools;
use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::env::FromString;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::database::AutoRetry;
use ockam_node::database::{Boolean, FromSqlxError, Nullable, SqlxDatabase, ToVoid};
use sqlx::any::AnyRow;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::any::AnyArgumentBuffer;
use sqlx::*;
use std::str::FromStr;
use std::sync::Arc;

use super::ProjectsRepository;

/// The ProjectsSqlxDatabase stores project information in several tables:
///
///  - project
///  - user_project
///  - user_role
///  - okta_config
///  - kafka_config
///
#[derive(Clone)]
pub struct ProjectsSqlxDatabase {
    database: SqlxDatabase,
}

impl ProjectsSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for projects");
        Self { database }
    }

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn ProjectsRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("projects").await?))
    }

    async fn is_shared_project(
        &self,
        users_roles: &[ProjectUserRole],
        transaction: &mut AnyConnection,
    ) -> Result<bool> {
        // There are three possible scenarios:
        // 1. The user enrolls and gets a project as an admin. There is an email from the "user"
        //      table that matches an email in the project's "user_role" field that has role "admin".
        // 2. Someone shares a project to the user via an invitation. There is an email from the "user"
        //      table that matches an email in the project's "user_role" field that has role "service_user".
        // 3. The user uses a ticket to enroll into a project. There is no email from the "user" table
        //      that matches an email in the project's "user_role" field.
        // This function returns true if we are in the second scenario; false otherwise.

        // Get emails from user_roles that are not admins
        if users_roles.is_empty() {
            return Ok(false);
        }

        let non_admin_emails: Vec<String> = users_roles
            .iter()
            .filter(|user_role| user_role.role != RoleInShare::Admin)
            .map(|user_role| user_role.email.to_string().to_lowercase())
            .unique()
            .collect();

        // Check if any of the emails are in the user table
        let q = format!(
            r#"SELECT EXISTS(SELECT 1 FROM "user" WHERE LOWER(email) IN ({}))"#,
            non_admin_emails
                .iter()
                .map(|e| format!("'{}'", e))
                .join(", ")
        );
        let shared: Boolean = query_scalar(&q).fetch_one(transaction).await.into_core()?;
        Ok(shared.to_bool())
    }

    async fn get_users_roles(
        &self,
        project_id: &str,
        transaction: &mut AnyConnection,
    ) -> Result<Vec<ProjectUserRole>> {
        let query = query_as("SELECT user_id, project_id, user_email, role, scope FROM user_role WHERE project_id = $1")
            .bind(project_id);
        let rows: Vec<UserRoleRow> = query.fetch_all(transaction).await.into_core()?;
        rows.into_iter().map(|r| r.project_user_role()).collect()
    }

    async fn set_as_default(
        &self,
        project_id: &str,
        transaction: &mut AnyConnection,
    ) -> Result<()> {
        let users_roles = self.get_users_roles(project_id, transaction).await?;
        if self
            .is_shared_project(&users_roles, &mut *transaction)
            .await?
        {
            return Err(Error::new(
                Origin::Api,
                Kind::Invalid,
                format!("the project {project_id} can't be set as default because is not owned by any local user"),
            ));
        }

        // set the project as the default one
        query("UPDATE project SET is_default = $1 WHERE project_id = $2")
            .bind(true)
            .bind(project_id)
            .execute(&mut *transaction)
            .await
            .void()?;

        // set all the others as non-default
        query("UPDATE project SET is_default = $1 WHERE project_id <> $2")
            .bind(false)
            .bind(project_id)
            .execute(&mut *transaction)
            .await
            .void()?;

        // set the associated space as default
        query("UPDATE space SET is_default = $1 WHERE space_id = (SELECT space_id FROM project WHERE project_id = $2)")
            .bind(true)
            .bind(project_id)
            .execute(&mut *transaction)
            .await
            .void()?;

        // set all the others as non-default
        query("UPDATE space SET is_default = $1 WHERE space_id <> (SELECT space_id FROM project WHERE project_id = $2)")
            .bind(false)
            .bind(project_id)
            .execute(&mut *transaction)
            .await
            .void()?;

        Ok(())
    }
}

#[async_trait]
impl ProjectsRepository for ProjectsSqlxDatabase {
    async fn store_project(&self, project: &ProjectModel) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        let is_default: bool;
        let mut project_name = &project.name;
        // If it's a shared project, it can't be set as default.
        if self
            .is_shared_project(&project.user_roles, &mut transaction)
            .await?
        {
            is_default = false;
            // Also, the name is set to the project id to avoid collisions with other
            // projects with the same name that belong to other spaces.
            project_name = &project.id;
        } else {
            // Set to default if there is no default project
            let default_project_id: Option<String> =
                query("SELECT project_id FROM project WHERE is_default = $1")
                    .bind(true)
                    .fetch_optional(&mut *transaction)
                    .await
                    .into_core()?
                    .map(|row| row.get(0));
            is_default =
                default_project_id.is_none() || default_project_id.as_ref() == Some(&project.id);
        }

        let query2 = query(
            r#"
            INSERT INTO project (project_id, project_name, is_default, space_id, space_name, project_identifier, project_change_history, access_route, authority_change_history, authority_access_route, version, running, operation_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (project_id)
            DO UPDATE SET project_name = $2, is_default = $3, space_id = $4, space_name = $5, project_identifier = $6, project_change_history = $7, access_route = $8, authority_change_history = $9, authority_access_route = $10, version = $11, running = $12, operation_id = $13"#,
        )
            .bind(&project.id)
            .bind(project_name)
            .bind(is_default)
            .bind(&project.space_id)
            .bind(&project.space_name)
            .bind(&project.identity)
            .bind(project.project_change_history.as_ref())
            .bind(&project.access_route)
            .bind(project.authority_identity.as_ref())
            .bind(project.authority_access_route.as_ref())
            .bind(project.version.as_ref())
            .bind(project.running.as_ref())
            .bind(project.operation_id.as_ref());
        query2.execute(&mut *transaction).await.void()?;

        if is_default {
            self.set_as_default(&project.id, &mut transaction).await?;
        }

        // remove any existing users related to that project if any
        let query3 = query("DELETE FROM user_project WHERE project_id = $1").bind(&project.id);
        query3.execute(&mut *transaction).await.void()?;

        // store the users associated to that project
        for user_email in &project.users {
            let query = query(
                r#"
            INSERT INTO user_project (user_email, project_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING"#,
            )
            .bind(user_email)
            .bind(&project.id);
            query.execute(&mut *transaction).await.void()?;
        }

        // remove any existing user roles related to that project if any
        let query4 = query("DELETE FROM user_role WHERE project_id = $1").bind(&project.id);
        query4.execute(&mut *transaction).await.void()?;

        // store the user roles associated to that project
        for user_role in &project.user_roles {
            let query = query(
                r#"
            INSERT INTO user_role (user_id, project_id, user_email, role, scope)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT DO NOTHING"#,
            )
            .bind(user_role.id as i64)
            .bind(&project.id)
            .bind(&user_role.email)
            .bind(&user_role.role)
            .bind(&user_role.scope);
            query.execute(&mut *transaction).await.void()?;
        }

        // make sure that the project space is also saved
        let query5 = query(
            r#"
          INSERT INTO space (space_id, space_name, is_default)
          VALUES ($1, $2, $3)
          ON CONFLICT (space_id)
          DO UPDATE SET space_name = $2, is_default = $3"#,
        )
        .bind(&project.space_id)
        .bind(&project.space_name)
        .bind(true);
        query5.execute(&mut *transaction).await.void()?;

        // store the okta configuration if any
        if let Some(okta_config) = &project.okta_config {
            let query = query(r#"
                INSERT INTO okta_config (project_id, tenant_base_url, client_id, certificate, attributes)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT DO NOTHING"#)
                .bind(&project.id)
                .bind(&okta_config.tenant_base_url)
                .bind(&okta_config.client_id)
                .bind(&okta_config.certificate)
                .bind(okta_config.attributes.join(",").to_string());
            query.execute(&mut *transaction).await.void()?;
        }

        // store the kafka configuration if any
        if let Some(kafka_config) = &project.kafka_config {
            let query = query(
                r#"
                INSERT INTO kafka_config (project_id, bootstrap_server)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING"#,
            )
            .bind(&project.id)
            .bind(&kafka_config.bootstrap_server);
            query.execute(&mut *transaction).await.void()?;
        }

        transaction.commit().await.void()
    }

    async fn get_project(&self, project_id: &str) -> Result<Option<ProjectModel>> {
        let query =
            query("SELECT project_name FROM project WHERE project_id = $1").bind(project_id);
        let row: Option<AnyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        match row {
            Some(r) => {
                let project_name: String = r.get(0);
                self.get_project_by_name(&project_name).await
            }
            None => Ok(None),
        }
    }

    async fn get_project_by_name(&self, name: &str) -> Result<Option<ProjectModel>> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query = query_as("SELECT project_id, project_name, is_default, space_id, space_name, project_identifier, project_change_history, access_route, authority_change_history, authority_access_route, version, running, operation_id FROM project WHERE project_name = $1").bind(name);
        let row: Option<ProjectRow> = query.fetch_optional(&mut *transaction).await.into_core()?;
        let project = match row.map(|r| r.project()).transpose()? {
            Some(mut project) => {
                // get the project users emails
                let query2 = query_as(
                    "SELECT project_id, user_email FROM user_project WHERE project_id = $1",
                )
                .bind(&project.id);
                let rows: Vec<UserProjectRow> =
                    query2.fetch_all(&mut *transaction).await.into_core()?;
                let users: Result<Vec<EmailAddress>> =
                    rows.into_iter().map(|r| r.user_email()).collect();
                project.users = users?;

                // get the project users roles
                let user_roles = self.get_users_roles(&project.id, &mut transaction).await?;
                project.user_roles = user_roles;

                // get the project okta configuration
                let query4 = query_as("SELECT project_id, tenant_base_url, client_id, certificate, attributes FROM okta_config WHERE project_id = $1")
                    .bind(&project.id);
                let row: Option<OktaConfigRow> =
                    query4.fetch_optional(&mut *transaction).await.into_core()?;
                project.okta_config = row.map(|r| r.okta_config()).transpose()?;

                // get the project kafka configuration
                let query5 = query_as(
                    "SELECT project_id, bootstrap_server FROM kafka_config WHERE project_id = $1",
                )
                .bind(&project.id);
                let row: Option<KafkaConfigRow> =
                    query5.fetch_optional(&mut *transaction).await.into_core()?;
                project.kafka_config = row.map(|r| r.kafka_config());

                Some(project)
            }

            None => None,
        };
        transaction.commit().await.void()?;
        Ok(project)
    }

    async fn get_projects(&self) -> Result<Vec<ProjectModel>> {
        let query = query("SELECT project_name FROM project");
        let rows: Vec<AnyRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        let project_names: Vec<String> = rows.iter().map(|r| r.get(0)).collect();
        let mut projects = vec![];
        for project_name in project_names {
            let project = self.get_project_by_name(&project_name).await?;
            if let Some(project) = project {
                projects.push(project);
            };
        }
        Ok(projects)
    }

    async fn get_default_project(&self) -> Result<Option<ProjectModel>> {
        let query = query("SELECT project_name FROM project WHERE is_default = $1").bind(true);
        let row: Option<AnyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        match row {
            Some(r) => {
                let project_name: String = r.get(0);
                self.get_project_by_name(&project_name).await
            }
            None => Ok(None),
        }
    }

    async fn set_default_project(&self, project_id: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        self.set_as_default(project_id, &mut transaction).await?;
        transaction.commit().await.void()
    }

    async fn delete_project(&self, project_id: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        // Check if the project is the default one
        let q = query_scalar(
            r#"SELECT EXISTS(SELECT 1 FROM project WHERE project_id = $1 AND is_default = $2)"#,
        )
        .bind(project_id.to_string())
        .bind(true);
        let is_default: Boolean = q.fetch_one(&mut *transaction).await.into_core()?;
        let is_default = is_default.to_bool();

        // Delete it
        let query1 = query("DELETE FROM project WHERE project_id = $1").bind(project_id);
        query1.execute(&mut *transaction).await.void()?;

        let query2 = query("DELETE FROM user_project WHERE project_id = $1").bind(project_id);
        query2.execute(&mut *transaction).await.void()?;

        let query3 = query("DELETE FROM user_role WHERE project_id = $1").bind(project_id);
        query3.execute(&mut *transaction).await.void()?;

        let query4 = query("DELETE FROM okta_config WHERE project_id = $1").bind(project_id);
        query4.execute(&mut *transaction).await.void()?;

        let query5 = query("DELETE FROM kafka_config WHERE project_id = $1").bind(project_id);
        query5.execute(&mut *transaction).await.void()?;

        // Set another project as default if the deleted one was the default
        if is_default {
            let project_ids: Vec<String> = query_scalar("SELECT project_id FROM project")
                .fetch_all(&mut *transaction)
                .await
                .into_core()?;
            for project_id in project_ids {
                if self
                    .set_as_default(&project_id, &mut transaction)
                    .await
                    .is_ok()
                {
                    break;
                }
            }
        }

        transaction.commit().await.void()?;
        Ok(())
    }
}

// Database serialization / deserialization

/// Low-level representation of a row in the projects table
#[derive(sqlx::FromRow)]
struct ProjectRow {
    project_id: String,
    project_name: String,
    #[allow(unused)]
    is_default: Boolean,
    space_id: String,
    space_name: String,
    project_identifier: Nullable<String>,
    project_change_history: Nullable<String>,
    access_route: String,
    authority_change_history: Nullable<String>,
    authority_access_route: Nullable<String>,
    version: Nullable<String>,
    running: Nullable<Boolean>,
    operation_id: Nullable<String>,
}

impl ProjectRow {
    pub(crate) fn project(&self) -> Result<ProjectModel> {
        self.complete_project(vec![], vec![], None, None)
    }

    pub(crate) fn complete_project(
        &self,
        user_emails: Vec<EmailAddress>,
        user_roles: Vec<ProjectUserRole>,
        okta_config: Option<OktaConfig>,
        kafka_config: Option<KafkaConfig>,
    ) -> Result<ProjectModel> {
        let project_identifier = self
            .project_identifier
            .to_option()
            .map(|i| Identifier::from_string(&i))
            .transpose()?;
        Ok(ProjectModel {
            id: self.project_id.clone(),
            name: self.project_name.clone(),
            space_id: self.space_id.clone(),
            space_name: self.space_name.clone(),
            identity: project_identifier,
            project_change_history: self.project_change_history.to_option(),
            access_route: self.access_route.clone(),
            authority_access_route: self.authority_access_route.to_option(),
            authority_identity: self.authority_change_history.to_option(),
            version: self.version.to_option(),
            running: self.running.to_option().map(|r| r.to_bool()),
            operation_id: self.operation_id.to_option(),
            users: user_emails,
            user_roles,
            okta_config,
            kafka_config,
        })
    }
}

/// Low-level representation of a row in the user_project table
#[derive(sqlx::FromRow)]
struct UserProjectRow {
    #[allow(unused)]
    project_id: String,
    user_email: String,
}

impl UserProjectRow {
    fn user_email(&self) -> Result<EmailAddress> {
        self.user_email.clone().try_into()
    }
}

/// Low-level representation of a row in the user_role table
#[derive(sqlx::FromRow)]
struct UserRoleRow {
    user_id: i64,
    #[allow(unused)]
    project_id: String,
    user_email: String,
    role: String,
    scope: String,
}

impl Type<Any> for EmailAddress {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for EmailAddress {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl Type<Any> for RoleInShare {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for RoleInShare {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl Type<Any> for ShareScope {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for ShareScope {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl Type<Any> for Url {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for Url {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl UserRoleRow {
    fn project_user_role(&self) -> Result<ProjectUserRole> {
        let role = RoleInShare::from_str(&self.role)
            .map_err(|e| Error::new(Origin::Api, Kind::Serialization, e.to_string()))?;
        let scope = ShareScope::from_str(&self.scope)
            .map_err(|e| Error::new(Origin::Api, Kind::Serialization, e.to_string()))?;
        Ok(ProjectUserRole {
            id: self.user_id as u64,
            email: self.user_email.clone().try_into()?,
            role,
            scope,
        })
    }
}

/// Low-level representation of a row in the okta_config table
#[derive(sqlx::FromRow)]
struct OktaConfigRow {
    #[allow(unused)]
    project_id: String,
    tenant_base_url: String,
    client_id: String,
    certificate: String,
    attributes: String,
}

impl OktaConfigRow {
    fn okta_config(&self) -> Result<OktaConfig> {
        let tenant_base_url = Url::parse(&self.tenant_base_url.clone())
            .map_err(|e| Error::new(Origin::Api, Kind::Serialization, e.to_string()))?;
        Ok(OktaConfig {
            tenant_base_url,
            certificate: self.certificate.clone(),
            client_id: self.client_id.clone(),
            attributes: self.attributes.split(',').map(|a| a.to_string()).collect(),
        })
    }
}

/// Low-level representation of a row in the kafka_config table
#[derive(sqlx::FromRow)]
struct KafkaConfigRow {
    #[allow(unused)]
    project_id: String,
    bootstrap_server: String,
}

impl KafkaConfigRow {
    fn kafka_config(&self) -> KafkaConfig {
        KafkaConfig {
            bootstrap_server: self.bootstrap_server.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::cli_state::{
        SpacesRepository, SpacesSqlxDatabase, UsersRepository, UsersSqlxDatabase,
    };
    use crate::cloud::enroll::auth0::UserInfo;
    use ockam_node::database::with_dbs;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn UsersRepository> = Arc::new(UsersSqlxDatabase::new(db.clone()));
            repository
                .store_user(&UserInfo {
                    sub: "sub".to_string(),
                    nickname: "nickname".to_string(),
                    name: "name".to_string(),
                    picture: "picture".to_string(),
                    updated_at: "2024-07-29T17:56:24.585Z".to_string(),
                    email: "Me@ockam.io".try_into().unwrap(),
                    email_verified: false,
                })
                .await
                .unwrap();

            let repository: Arc<dyn ProjectsRepository> = Arc::new(ProjectsSqlxDatabase::new(db));

            // create and store 3 projects
            let project1 = create_project(
                "1",
                "1",
                vec!["me@ockam.io", "him@ockam.io"],
                vec![
                    create_project_user_role(2, RoleInShare::Guest, "guest@ockam.io"),
                    create_project_user_role(3, RoleInShare::Service, "me@ockam.io"),
                ],
            );
            let project2 = create_project(
                "2",
                "name2",
                vec!["me@ockam.io", "you@ockam.io"],
                vec![
                    create_project_user_role(1, RoleInShare::Admin, None),
                    create_project_user_role(2, RoleInShare::Guest, None),
                ],
            );
            let mut project3 = create_project(
                "3",
                "name3",
                vec!["me@ockam.io", "him@ockam.io", "her@ockam.io"],
                vec![
                    create_project_user_role(1, RoleInShare::Admin, None),
                    create_project_user_role(2, RoleInShare::Guest, None),
                ],
            );
            // The first project is a shared project; shouldn't be set as default
            repository.store_project(&project1).await?;
            let result = repository.get_default_project().await?;
            assert!(result.is_none());

            // The first owned stored project is the default one
            repository.store_project(&project2).await?;
            let result = repository.get_default_project().await?;
            assert_eq!(result, Some(project2.clone()));

            repository.store_project(&project3).await?;

            // retrieve them as a list or by name
            let result = repository.get_projects().await?;
            for project in vec![project1.clone(), project2.clone(), project3.clone()] {
                assert!(result.contains(&project));
            }

            let result = repository.get_project_by_name("name2").await?;
            assert_eq!(result, Some(project2.clone()));

            // a project can be marked as the default project
            repository.set_default_project("3").await?;
            let result = repository.get_default_project().await?;
            assert_eq!(result, Some(project3.clone()));

            // updating a project which was already the default should keep it the default
            project3.users = vec!["someone@ockam.io".try_into().unwrap()];
            repository.store_project(&project3).await?;
            let result = repository.get_default_project().await?;
            assert_eq!(result, Some(project3.clone()));

            // a project can be deleted
            repository.delete_project("3").await?;

            // if the default project is deleted, another one should be set as default
            let result = repository.get_default_project().await?;
            assert_eq!(result, Some(project2.clone()));

            let result = repository.get_projects().await?;
            for project in vec![project1.clone(), project2.clone()] {
                assert!(result.contains(&project));
            }

            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_store_project_space() -> Result<()> {
        with_dbs(|db| async move {
            let projects_repository: Arc<dyn ProjectsRepository> =
                Arc::new(ProjectsSqlxDatabase::new(db.clone()));

            let project = create_project("1", "name1", vec![], vec![]);
            projects_repository.store_project(&project).await?;

            // the space information coming from the project must also be stored in the spaces table
            let spaces_repository: Arc<dyn SpacesRepository> =
                Arc::new(SpacesSqlxDatabase::new(db));
            let space = spaces_repository.get_default_space().await?.unwrap();
            assert_eq!(project.space_id, space.id);
            assert_eq!(project.space_name, space.name);

            Ok(())
        })
        .await
    }

    /// HELPERS
    fn create_project(
        id: &str,
        name: &str,
        user_emails: Vec<&str>,
        user_roles: Vec<ProjectUserRole>,
    ) -> ProjectModel {
        ProjectModel {
            id: id.into(),
            name: name.into(),
            space_id: "space-id".into(),
            space_name: "space-name".into(),
            access_route: "route".into(),
            users: user_emails
                .iter()
                .map(|u| u.to_string().try_into().unwrap())
                .collect(),
            identity: Some(
                Identifier::from_str(
                    "I124ed0b2e5a2be82e267ead6b3279f683616b66da1b2c3d4e5f6a6b5c4d3e2f1",
                )
                .unwrap(),
            ),
            project_change_history: Some("project-identity".into()),
            authority_access_route: Some("authority-route".into()),
            authority_identity: Some("authority-identity".into()),
            okta_config: Some(create_okta_config()),
            kafka_config: Some(create_kafka_config()),
            version: Some("1.0".into()),
            running: Some(true),
            operation_id: Some("abc".into()),
            user_roles,
        }
    }

    fn create_project_user_role<'a, E: Into<Option<&'a str>>>(
        user_id: u64,
        role: RoleInShare,
        email: E,
    ) -> ProjectUserRole {
        let email = match email.into() {
            Some(email) => email,
            None => match role {
                RoleInShare::Admin => "me@ockam.io",
                RoleInShare::Service => "service@ockam.com",
                RoleInShare::Guest => "guest@ockam.com",
            },
        }
        .try_into()
        .unwrap();
        ProjectUserRole {
            email,
            id: user_id,
            role,
            scope: ShareScope::Project,
        }
    }

    fn create_okta_config() -> OktaConfig {
        OktaConfig {
            tenant_base_url: Url::parse("http://ockam.io").unwrap(),
            certificate: "certificate".to_string(),
            client_id: "client-id".to_string(),
            attributes: vec!["attribute1".into(), "attribute2".into()],
        }
    }

    fn create_kafka_config() -> KafkaConfig {
        KafkaConfig {
            bootstrap_server: "bootstrap_server".to_string(),
        }
    }
}
