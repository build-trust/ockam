use std::str::FromStr;

use sqlx::any::AnyRow;
use sqlx::*;

use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::env::FromString;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::database::{
    Boolean, FromSqlxError, Nullable, SqlxDatabase, SqlxType, ToSqlxType, ToVoid,
};

use crate::cloud::addon::KafkaConfig;
use crate::cloud::email_address::EmailAddress;
use crate::cloud::project::models::{OktaConfig, ProjectModel, ProjectUserRole};
use crate::cloud::share::{RoleInShare, ShareScope};
use crate::minicbor_url::Url;

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

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("projects").await?))
    }
}

#[async_trait]
impl ProjectsRepository for ProjectsSqlxDatabase {
    async fn store_project(&self, project: &ProjectModel) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query1 = query_scalar(
            "SELECT EXISTS(SELECT 1 FROM project WHERE is_default=$1 AND project_id=$2)",
        )
        .bind(true)
        .bind(project.id.clone());
        let is_already_default: i32 = query1.fetch_one(&mut *transaction).await.into_core()?;
        let is_already_default = is_already_default == 1;

        let query2 = query(
            "INSERT OR REPLACE INTO project (project_id, project_name, is_default, space_id, space_name, project_identifier, project_change_history, access_route, authority_change_history, authority_access_route, version, running, operation_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
            .bind(&project.id)
            .bind(&project.name)
            .bind(is_already_default)
            .bind(&project.space_id)
            .bind(&project.space_name)
            .bind(project.identity.as_ref().map(|i| i.to_sql()))
            .bind(project.project_change_history.as_ref())
            .bind(&project.access_route)
            .bind(project.authority_identity.as_ref())
            .bind(project.authority_access_route.as_ref())
            .bind(project.version.as_ref())
            .bind(project.running.as_ref())
            .bind(project.operation_id.as_ref());
        query2.execute(&mut *transaction).await.void()?;

        // remove any existing users related to that project if any
        let query3 = query("DELETE FROM user_project WHERE project_id=$1").bind(&project.id);
        query3.execute(&mut *transaction).await.void()?;

        // store the users associated to that project
        for user_email in &project.users {
            let query = query("INSERT OR REPLACE INTO user_project VALUES (?, ?)")
                .bind(user_email.to_sql())
                .bind(&project.id);
            query.execute(&mut *transaction).await.void()?;
        }

        // remove any existing user roles related to that project if any
        let query4 = query("DELETE FROM user_role WHERE project_id=$1").bind(&project.id);
        query4.execute(&mut *transaction).await.void()?;

        // store the user roles associated to that project
        for user_role in &project.user_roles {
            let query = query("INSERT OR REPLACE INTO user_role VALUES (?, ?, ?, ?, ?)")
                .bind(user_role.id.to_sql())
                .bind(&project.id)
                .bind(user_role.email.to_sql())
                .bind(user_role.role.to_string())
                .bind(user_role.scope.to_string());
            query.execute(&mut *transaction).await.void()?;
        }

        // make sure that the project space is also saved
        let query5 = query("INSERT OR IGNORE INTO space VALUES ($1, $2, $3)")
            .bind(&project.space_id)
            .bind(&project.space_name)
            .bind(true);
        query5.execute(&mut *transaction).await.void()?;

        // store the okta configuration if any
        for okta_config in &project.okta_config {
            let query = query("INSERT OR REPLACE INTO okta_config VALUES (?, ?, ?, ?, ?)")
                .bind(&project.id)
                .bind(okta_config.tenant_base_url.to_string())
                .bind(&okta_config.client_id)
                .bind(&okta_config.certificate)
                .bind(okta_config.attributes.join(",").to_string());
            query.execute(&mut *transaction).await.void()?;
        }

        // store the kafka configuration if any
        for kafka_config in &project.kafka_config {
            let query = query("INSERT OR REPLACE INTO kafka_config VALUES (?, ?)")
                .bind(&project.id)
                .bind(&kafka_config.bootstrap_server);
            query.execute(&mut *transaction).await.void()?;
        }

        transaction.commit().await.void()
    }

    async fn get_project(&self, project_id: &str) -> Result<Option<ProjectModel>> {
        let query = query("SELECT project_name FROM project WHERE project_id=$1").bind(project_id);
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

        let query = query_as("SELECT project_id, project_name, is_default, space_id, space_name, project_identifier, project_change_history, access_route, authority_change_history, authority_access_route, version, running, operation_id FROM project WHERE project_name=$1").bind(name);
        let row: Option<ProjectRow> = query.fetch_optional(&mut *transaction).await.into_core()?;
        let project = match row.map(|r| r.project()).transpose()? {
            Some(mut project) => {
                // get the project users emails
                let query2 =
                    query_as("SELECT project_id, user_email FROM user_project WHERE project_id=$1")
                        .bind(&project.id);
                let rows: Vec<UserProjectRow> =
                    query2.fetch_all(&mut *transaction).await.into_core()?;
                let users: Result<Vec<EmailAddress>> =
                    rows.into_iter().map(|r| r.user_email()).collect();
                project.users = users?;

                // get the project users roles
                let query3 = query_as("SELECT user_id, project_id, user_email, role, scope FROM user_role WHERE project_id=$1")
                    .bind(&project.id);
                let rows: Vec<UserRoleRow> =
                    query3.fetch_all(&mut *transaction).await.into_core()?;
                let user_roles: Vec<ProjectUserRole> = rows
                    .into_iter()
                    .map(|r| r.project_user_role())
                    .collect::<Result<Vec<_>>>()?;
                project.user_roles = user_roles;

                // get the project okta configuration
                let query4 = query_as("SELECT project_id, tenant_base_url, client_id, certificate, attributes FROM okta_config WHERE project_id=$1")
                    .bind(&project.id);
                let row: Option<OktaConfigRow> =
                    query4.fetch_optional(&mut *transaction).await.into_core()?;
                project.okta_config = row.map(|r| r.okta_config()).transpose()?;

                // get the project kafka configuration
                let query5 = query_as(
                    "SELECT project_id, bootstrap_server FROM kafka_config WHERE project_id=$1",
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
        let query = query("SELECT project_name FROM project WHERE is_default=$1").bind(true);
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
        // set the project as the default one
        let query1 = query("UPDATE project SET is_default = ? WHERE project_id = ?")
            .bind(true)
            .bind(project_id);
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE project SET is_default = ? WHERE project_id <> ?")
            .bind(false)
            .bind(project_id);
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }

    async fn delete_project(&self, project_id: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query1 = query("DELETE FROM project WHERE project_id=?").bind(project_id);
        query1.execute(&mut *transaction).await.void()?;

        let query2 = query("DELETE FROM user_project WHERE project_id=?").bind(project_id);
        query2.execute(&mut *transaction).await.void()?;

        let query3 = query("DELETE FROM user_role WHERE project_id=?").bind(project_id);
        query3.execute(&mut *transaction).await.void()?;

        let query4 = query("DELETE FROM okta_config WHERE project_id=?").bind(project_id);
        query4.execute(&mut *transaction).await.void()?;

        let query5 = query("DELETE FROM kafka_config WHERE project_id=?").bind(project_id);
        query5.execute(&mut *transaction).await.void()?;

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

impl ToSqlxType for EmailAddress {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Text(self.to_string())
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

    use crate::cli_state::{SpacesRepository, SpacesSqlxDatabase};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // create and store 2 projects
        let project1 = create_project(
            "1",
            "name1",
            vec!["me@ockam.io", "you@ockam.io"],
            vec![
                create_project_user_role(1, RoleInShare::Admin),
                create_project_user_role(2, RoleInShare::Guest),
            ],
        );
        let mut project2 = create_project(
            "2",
            "name2",
            vec!["me@ockam.io", "him@ockam.io", "her@ockam.io"],
            vec![
                create_project_user_role(1, RoleInShare::Admin),
                create_project_user_role(2, RoleInShare::Guest),
            ],
        );
        repository.store_project(&project1).await?;
        repository.store_project(&project2).await?;

        // retrieve them as a list or by name
        let result = repository.get_projects().await?;
        assert_eq!(result, vec![project1.clone(), project2.clone()]);

        let result = repository.get_project_by_name("name1").await?;
        assert_eq!(result, Some(project1.clone()));

        // a project can be marked as the default project
        repository.set_default_project("1").await?;
        let result = repository.get_default_project().await?;
        assert_eq!(result, Some(project1.clone()));

        repository.set_default_project("2").await?;
        let result = repository.get_default_project().await?;
        assert_eq!(result, Some(project2.clone()));

        // updating a project which was already the default should keep it the default
        project2.users = vec!["someone@ockam.io".try_into().unwrap()];
        repository.store_project(&project2).await?;
        let result = repository.get_default_project().await?;
        assert_eq!(result, Some(project2.clone()));

        // a project can be deleted
        repository.delete_project("2").await?;
        let result = repository.get_default_project().await?;
        assert_eq!(result, None);

        let result = repository.get_projects().await?;
        assert_eq!(result, vec![project1.clone()]);
        Ok(())
    }

    #[tokio::test]
    async fn test_store_project_space() -> Result<()> {
        let db = SqlxDatabase::in_memory("projects").await?;
        let projects_repository = ProjectsSqlxDatabase::new(db.clone());
        let project = create_project("1", "name1", vec![], vec![]);
        projects_repository.store_project(&project).await?;

        // the space information coming from the project must also be stored in the spaces table
        let spaces_repository: Arc<dyn SpacesRepository> = Arc::new(SpacesSqlxDatabase::new(db));
        let space = spaces_repository.get_default_space().await?.unwrap();
        assert_eq!(project.space_id, space.id);
        assert_eq!(project.space_name, space.name);

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn ProjectsRepository>> {
        Ok(Arc::new(ProjectsSqlxDatabase::create().await?))
    }

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

    fn create_project_user_role(user_id: u64, role: RoleInShare) -> ProjectUserRole {
        ProjectUserRole {
            email: "user@email".try_into().unwrap(),
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
