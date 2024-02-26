/// This migration adds a node name column to the identity attributes table
pub mod migration_20231231100000_node_name_identity_attributes;
/// This migration moves attributes from identity_attributes to the authority_member table for authority nodes
pub mod migration_20240111100001_add_authority_tables;
/// This migration updates policies to not rely on trust_context_id,
/// also introduces `node_name` and  replicates policy for each existing node
pub mod migration_20240111100002_delete_trust_context;
/// This migration moves policies attached to resource types from
/// table "resource_policy" to "resource_type_policy"
pub mod migration_20240212100000_split_policies;
