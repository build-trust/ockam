use crate::database::migrations::RustMigration;
use crate::database::{FromSqlxError, ToSqlxType, ToVoid};
use core::fmt;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::{async_trait, Result};
use regex::Regex;
use sqlx::*;

/// This migration updates policies to not rely on trust_context_id,
/// also introduces `node_name` and  replicates policy for each existing node
#[derive(Debug)]
pub struct PolicyTrustContextId;

#[async_trait]
impl RustMigration for PolicyTrustContextId {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> i64 {
        Self::version()
    }

    async fn migrate(&self, connection: &mut SqliteConnection) -> Result<bool> {
        Self::migrate_update_policies(connection).await
    }
}

impl PolicyTrustContextId {
    /// Migration version
    pub fn version() -> i64 {
        20240111100002
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20240111100002_delete_trust_context"
    }

    /// This migration updates policies to not rely on trust_context_id,
    /// also introduces `node_name` and  replicates policy for each existing node
    pub(crate) async fn migrate_update_policies(connection: &mut SqliteConnection) -> Result<bool> {
        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;

        let query_node_names = query_as("SELECT name FROM node");
        let node_names: Vec<NodeNameRow> = query_node_names
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;
        let node_names = node_names.into_iter().map(|r| r.name).collect::<Vec<_>>();

        let query_policies = query_as("SELECT resource, action, expression FROM policy_old");
        let rows: Vec<PolicyRow> = query_policies
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;

        for row in rows {
            let expression = {
                let expression: Expr = minicbor::decode(&row.expression)?;
                Self::update_expression(&expression)
            };
            for node_name in &node_names {
                let insert = query("INSERT INTO policy (resource, action, expression, node_name) VALUES (?, ?, ?, ?)")
                    .bind(row.resource.to_sql())
                    .bind(row.action.to_sql())
                    .bind(expression.to_sql())
                    .bind(node_name.to_sql());

                insert.execute(&mut *transaction).await.void()?;
            }
        }

        // finally drop the old table
        query("DROP TABLE policy_old")
            .execute(&mut *transaction)
            .await
            .void()?;

        transaction.commit().await.void()?;

        Ok(true)
    }

    fn update_expression(expression: &Expr) -> String {
        let expression = expression.to_string();

        let regex = Regex::new(r#"\(= subject.trust_context_id "[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}"\)"#).unwrap();
        let expression = regex.replace(&expression, "subject.has_credential");

        expression.replace(
            "(= resource.trust_context_id subject.trust_context_id)",
            "subject.has_credential",
        )
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum Expr {
    #[n(1)] Str   (#[n(0)] String),
    #[n(2)] Int   (#[n(0)] i64),
    #[n(3)] Float (#[n(0)] f64),
    #[n(4)] Bool  (#[n(0)] bool),
    #[n(5)] Ident (#[n(0)] String),
    #[n(6)] Seq   (#[n(0)] Vec<Expr>),
    #[n(7)] List  (#[n(0)] Vec<Expr>)
}

#[rustfmt::skip]
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /// Control stack element.
        enum Op<'a> {
            Show(&'a Expr),
            ListEnd,
            SeqEnd,
            Whitespace,
        }

        // Control stack.
        let mut ctrl = vec![Op::Show(self)];

        while let Some(e) = ctrl.pop() {
            match e {
                Op::Show(Expr::Str(s)) => write!(f, "{s:?}")?,
                Op::Show(Expr::Int(i)) => write!(f, "{i}")?,
                Op::Show(Expr::Float(x)) => {
                    if x.is_nan() {
                        f.write_str("nan")?
                    } else if x.is_infinite() {
                        if x.is_sign_negative() {
                            f.write_str("-inf")?
                        } else {
                            f.write_str("+inf")?
                        }
                    } else {
                        write!(f, "{x:?}")?
                    }
                }
                Op::Show(Expr::Bool(b)) => write!(f, "{b}")?,
                Op::Show(Expr::Ident(v)) => f.write_str(v)?,
                Op::Show(Expr::List(es)) => {
                    ctrl.push(Op::ListEnd);
                    f.write_str("(")?;
                    let mut n = es.len();
                    for e in es.iter().rev() {
                        ctrl.push(Op::Show(e));
                        if n > 1 {
                            ctrl.push(Op::Whitespace)
                        }
                        n -= 1
                    }
                }
                Op::Show(Expr::Seq(es)) => {
                    ctrl.push(Op::SeqEnd);
                    f.write_str("[")?;
                    let mut n = es.len();
                    for e in es.iter().rev() {
                        ctrl.push(Op::Show(e));
                        if n > 1 {
                            ctrl.push(Op::Whitespace)
                        }
                        n -= 1
                    }
                }
                Op::ListEnd    => f.write_str(")")?,
                Op::SeqEnd     => f.write_str("]")?,
                Op::Whitespace => f.write_str(" ")?,
            }
        }

        Ok(())
    }
}

#[derive(FromRow)]
struct NodeNameRow {
    name: String,
}

// Low-level representation of a table row before data migration
#[derive(FromRow)]
struct PolicyRow {
    resource: String,
    action: String,
    expression: Vec<u8>,
}

#[cfg(test)]
mod test {
    use crate::database::migrations::node_migration_set::NodeMigrationSet;
    use crate::database::{MigrationSet, SqlxDatabase};
    use sqlx::query::Query;
    use sqlx::sqlite::SqliteArguments;
    use tempfile::NamedTempFile;

    use super::*;

    #[derive(FromRow)]
    struct PolicyRowNew {
        resource: String,
        action: String,
        expression: String,
        node_name: String,
    }

    #[tokio::test]
    async fn update_expression() -> Result<()> {
        // resource.trust_context_id == subject.trust_context_id
        let old_expression1 = hex::decode("82078183820581613D82058178197265736F757263652E74727573745F636F6E746578745F696482058178187375626A6563742E74727573745F636F6E746578745F6964").unwrap();
        let old_expression1: Expr = minicbor::decode(&old_expression1)?;
        let new_expression1 = PolicyTrustContextId::update_expression(&old_expression1);
        assert_eq!("subject.has_credential", new_expression1);

        // subject.trust_context_id == a994262c-4d59-4756-a158-19db6994feed
        let old_expression2 = hex::decode("82078183820581613D82058178187375626A6563742E74727573745F636F6E746578745F6964820181782461393934323632632D346435392D343735362D613135382D313964623639393466656564").unwrap();
        let old_expression2: Expr = minicbor::decode(&old_expression2)?;
        let new_expression2 = PolicyTrustContextId::update_expression(&old_expression2);
        assert_eq!("subject.has_credential", new_expression2);

        Ok(())
    }

    #[tokio::test]
    async fn test_migration() -> Result<()> {
        // create the database pool and migrate the tables
        let db_file = NamedTempFile::new().unwrap();

        let pool = SqlxDatabase::create_connection_pool(db_file.path()).await?;

        let mut connection = pool.acquire().await.into_core()?;

        NodeMigrationSet
            .create_migrator()?
            .migrate_up_to_skip_last_rust_migration(&pool, PolicyTrustContextId::version())
            .await?;

        let insert_node1 = insert_node("n1".to_string());
        let insert_node2 = insert_node("n2".to_string());

        // true
        let expr1 = hex::decode("820481f5").unwrap();
        let insert1 = insert_policy("R1".to_string(), "A1".to_string(), expr1);

        // subject.trust_context_id == a994262c-4d59-4756-a158-19db6994feed
        let expr2 = hex::decode("82078183820581613D82058178187375626A6563742E74727573745F636F6E746578745F6964820181782461393934323632632D346435392D343735362D613135382D313964623639393466656564").unwrap();
        let insert2 = insert_policy("R2".to_string(), "A2".to_string(), expr2);

        // resource.trust_context_id == subject.trust_context_id && subject.ockam-role == enroller
        let expr3 = hex::decode("8207818382058163616e6482078183820581613d82058178197265736f757263652e74727573745f636f6e746578745f696482058178187375626a6563742e74727573745f636f6e746578745f696482078183820581613d820581727375626a6563742e6f636b616d2d726f6c6582018168656e726f6c6c6572").unwrap();
        let insert3 = insert_policy("R3".to_string(), "A3".to_string(), expr3);

        insert_node1.execute(&pool).await.void()?;
        insert_node2.execute(&pool).await.void()?;
        insert1.execute(&pool).await.void()?;
        insert2.execute(&pool).await.void()?;
        insert3.execute(&pool).await.void()?;

        // apply migrations
        NodeMigrationSet
            .create_migrator()?
            .migrate_up_to(&pool, PolicyTrustContextId::version())
            .await?;

        for node_name in &["n1", "n2"] {
            let rows: Vec<PolicyRowNew> = query_as(
                "SELECT resource, action, expression, node_name FROM policy WHERE node_name = ?",
            )
            .bind(node_name.to_sql())
            .fetch_all(&mut *connection)
            .await
            .into_core()?;

            assert_eq!(rows.len(), 3);

            assert_eq!(&rows[0].node_name, node_name);
            assert_eq!(rows[0].resource, "R1");
            assert_eq!(rows[0].action, "A1");
            assert_eq!(rows[0].expression, "true");

            assert_eq!(&rows[1].node_name, node_name);
            assert_eq!(rows[1].resource, "R2");
            assert_eq!(rows[1].action, "A2");
            assert_eq!(rows[1].expression, "subject.has_credential");

            assert_eq!(&rows[2].node_name, node_name);
            assert_eq!(rows[2].resource, "R3");
            assert_eq!(rows[2].action, "A3");
            assert_eq!(
                rows[2].expression,
                r#"(and subject.has_credential (= subject.ockam-role "enroller"))"#
            );
        }

        Ok(())
    }

    /// HELPERS
    fn insert_policy(
        resource: String,
        action: String,
        expression: Vec<u8>,
    ) -> Query<'static, Sqlite, SqliteArguments<'static>> {
        query("INSERT INTO policy_old (resource, action, expression) VALUES (?, ?, ?)")
            .bind(resource.to_sql())
            .bind(action.to_sql())
            .bind(expression.to_sql())
    }

    fn insert_node(name: String) -> Query<'static, Sqlite, SqliteArguments<'static>> {
        query("INSERT INTO node (name, identifier, verbosity, is_default, is_authority) VALUES (?, ?, ?, ?, ?)")
            .bind(name.to_sql())
            .bind("I_TEST".to_string().to_sql())
            .bind(1.to_sql())
            .bind(0.to_sql())
            .bind(false.to_sql())
    }
}
