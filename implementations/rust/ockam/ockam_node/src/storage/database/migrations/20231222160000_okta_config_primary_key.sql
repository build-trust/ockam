-- add a unique constraint to the okta_config table
-- and remove duplicate rows
DELETE FROM okta_config WHERE
    rowid NOT IN (
        SELECT min(rowid) FROM okta_config
        GROUP BY project_id, tenant_base_url, client_id
    );
CREATE UNIQUE INDEX IF NOT EXISTS okta_config_index ON okta_config (project_id, tenant_base_url, client_id);
