------------------------------------
-- TRUNCATE ALL TABLES BY TENANT ID
------------------------------------

CREATE OR REPLACE PROCEDURE delete_table_data_by_tenant_id(tablename TEXT, tenant_id TEXT)
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = tablename
    ) THEN
        EXECUTE format('DELETE FROM %I WHERE tenant_id = %L', tablename, tenant_id);
    END IF;
END;
$$;

CREATE OR REPLACE PROCEDURE delete_tables_data_by_tenant_id(tenant_id TEXT)
LANGUAGE plpgsql
AS $$
BEGIN
    CALL delete_table_data_by_tenant_id('user_role', tenant_id);
    CALL delete_table_data_by_tenant_id('user_project', tenant_id);
    CALL delete_table_data_by_tenant_id('user_space', tenant_id);
    CALL delete_table_data_by_tenant_id('user', tenant_id);
    CALL delete_table_data_by_tenant_id('subscription', tenant_id);
    CALL delete_table_data_by_tenant_id('project_journey', tenant_id);
    CALL delete_table_data_by_tenant_id('host_journey', tenant_id);
    CALL delete_table_data_by_tenant_id('project', tenant_id);
    CALL delete_table_data_by_tenant_id('space', tenant_id);
    CALL delete_table_data_by_tenant_id('resource_policy', tenant_id);
    CALL delete_table_data_by_tenant_id('resource_type_policy', tenant_id);
    CALL delete_table_data_by_tenant_id('resource', tenant_id);
    CALL delete_table_data_by_tenant_id('authority_enrollment_token', tenant_id);
    CALL delete_table_data_by_tenant_id('authority_member', tenant_id);
    CALL delete_table_data_by_tenant_id('secure_channel', tenant_id);
    CALL delete_table_data_by_tenant_id('identity_enrollment', tenant_id);
    CALL delete_table_data_by_tenant_id('identity_attributes', tenant_id);
    CALL delete_table_data_by_tenant_id('purpose_key', tenant_id);
    CALL delete_table_data_by_tenant_id('credential', tenant_id);
    CALL delete_table_data_by_tenant_id('named_identity', tenant_id);
    CALL delete_table_data_by_tenant_id('identity', tenant_id);
    CALL delete_table_data_by_tenant_id('tcp_inlet', tenant_id);
    CALL delete_table_data_by_tenant_id('tcp_outlet_status', tenant_id);
    CALL delete_table_data_by_tenant_id('node', tenant_id);
    CALL delete_table_data_by_tenant_id('vault', tenant_id);
    CALL delete_table_data_by_tenant_id('signing_secret', tenant_id);
    CALL delete_table_data_by_tenant_id('x25519_secret', tenant_id);
    CALL delete_table_data_by_tenant_id('aead_secret', tenant_id);
    CALL delete_table_data_by_tenant_id('okta_config', tenant_id);
    CALL delete_table_data_by_tenant_id('kafka_config', tenant_id);
END;
$$;
