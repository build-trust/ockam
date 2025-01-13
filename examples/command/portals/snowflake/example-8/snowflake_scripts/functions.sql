-- These functions are dependent the service, and needs to be created after the service is created

USE ROLE MSSQL_CONNECTOR_ROLE;
USE DATABASE MSSQL_CONNECTOR_DB;
USE WAREHOUSE MSSQL_CONNECTOR_WH;
USE SCHEMA MSSQL_CONNECTOR_SCHEMA;

-- Copy function
CREATE OR REPLACE FUNCTION _ockam_copy_from_mssql(SOURCE_TABLE STRING, TARGET_TABLE STRING)
    RETURNS VARCHAR
    CALLED ON NULL INPUT
    VOLATILE
    SERVICE = MSSQL_CONNECTOR_CLIENT
    ENDPOINT = 'http-endpoint'
    AS '/copy_to_snowflake';

-- Copy procedure wrapper
CREATE OR REPLACE PROCEDURE ockam_mssql_copy(SOURCE_TABLE STRING, TARGET_TABLE STRING)
    RETURNS STRING
    LANGUAGE PYTHON
    RUNTIME_VERSION = '3.11'
    PACKAGES = ('snowflake-snowpark-python')
    HANDLER = 'wrap_copy'
    EXECUTE AS OWNER
AS '
def wrap_copy(session, source_table, target_table):
    return session.sql(f"SELECT _ockam_copy_from_mssql($${source_table}$$, $${target_table}$$);").collect()[0][0]
';
