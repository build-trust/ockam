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
/// This migration removes orphan resources
pub mod migration_20240313100000_remove_orphan_resources;
/// This migration updates the policy expressions so that they start with an operator
pub mod migration_20240503100000_update_policy_expressions;
/// This migration adds an authority_id to the authority_members table to track the authority having members.
pub mod migration_20250114100000_members_authority_id;
