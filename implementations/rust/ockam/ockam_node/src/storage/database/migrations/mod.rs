/// This module contains support code for migrating databases
pub mod sqlx_migration;

pub(super) mod common;

/// This migration adds a node name column to the identity attributes table
pub mod migration_20231231100000_node_name_identity_attributes;
/// This migration moves attributes from identity_attributes to the authority_member table for authority nodes
pub mod migration_20240111100001_add_authority_tables;
