--------------------------------------------
-- TRUNCATE ALL TABLES FOR THE CURRENT USER
--------------------------------------------

CREATE OR REPLACE PROCEDURE delete_if_table_exists(tablename TEXT)
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = tablename
    ) THEN
        EXECUTE format('DELETE FROM %I', tablename);
END IF;
END;
$$;

CREATE OR REPLACE PROCEDURE delete_tenant_tables()
LANGUAGE plpgsql
AS $$
BEGIN
    CALL delete_if_table_exists('user_role');
    CALL delete_if_table_exists('user_project');
    CALL delete_if_table_exists('user_space');
    CALL delete_if_table_exists('user');
    CALL delete_if_table_exists('subscription');
    CALL delete_if_table_exists('project_journey');
    CALL delete_if_table_exists('host_journey');
    CALL delete_if_table_exists('project');
    CALL delete_if_table_exists('space');
    CALL delete_if_table_exists('resource_policy');
    CALL delete_if_table_exists('resource_type_policy');
    CALL delete_if_table_exists('resource');
    CALL delete_if_table_exists('authority_enrollment_token');
    CALL delete_if_table_exists('authority_member');
    CALL delete_if_table_exists('secure_channel');
    CALL delete_if_table_exists('identity_enrollment');
    CALL delete_if_table_exists('identity_attributes');
    CALL delete_if_table_exists('purpose_key');
    CALL delete_if_table_exists('credential');
    CALL delete_if_table_exists('named_identity');
    CALL delete_if_table_exists('identity');
    CALL delete_if_table_exists('tcp_inlet');
    CALL delete_if_table_exists('tcp_outlet_status');
    CALL delete_if_table_exists('node');
    CALL delete_if_table_exists('vault');
    CALL delete_if_table_exists('signing_secret');
    CALL delete_if_table_exists('x25519_secret');
    CALL delete_if_table_exists('aead_secret');
    CALL delete_if_table_exists('okta_config');
    CALL delete_if_table_exists('kafka_config');
END;
$$;
