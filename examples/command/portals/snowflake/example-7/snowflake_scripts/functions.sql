-- These functions are dependent the service, and needs to be created after the service is created

USE ROLE MSSQL_API_ROLE;
USE DATABASE MSSQL_API_DB;
USE WAREHOUSE MSSQL_API_WH;
USE SCHEMA MSSQL_API_SCHEMA;

-- Query
CREATE OR REPLACE FUNCTION _ockam_query_mssql(query STRING)
    RETURNS VARCHAR
    CALLED ON NULL INPUT
    VOLATILE
    SERVICE = MSSQL_API_CLIENT
    ENDPOINT = 'http-endpoint'
    AS '/query';


CREATE OR REPLACE PROCEDURE ockam_mssql_query(QUERY STRING)
    RETURNS TABLE()
    LANGUAGE PYTHON
    RUNTIME_VERSION = '3.11'
    PACKAGES = ('snowflake-snowpark-python')
    HANDLER = 'wrap_query'
    EXECUTE AS OWNER
AS '
import json

def wrap_query(session, query):
    data = json.loads(session.sql(f"SELECT _ockam_query_mssql($${query}$$);").collect()[0][0])
    keys = data[0]
    values = data[1:]
    return session.create_dataframe(values).to_df(keys)
';

-- Execute Statement
CREATE OR REPLACE FUNCTION _ockam_mssql_execute(QUERY STRING)
    RETURNS VARCHAR
    CALLED ON NULL INPUT
    VOLATILE
    SERVICE = MSSQL_API_CLIENT
    ENDPOINT = 'http-endpoint'
    AS '/execute';


CREATE OR REPLACE PROCEDURE ockam_mssql_execute(QUERY STRING)
    RETURNS STRING
    LANGUAGE PYTHON
    RUNTIME_VERSION = '3.11'
    PACKAGES = ('snowflake-snowpark-python')
    HANDLER = 'wrap_execute'
    EXECUTE AS OWNER
AS '
def wrap_execute(session, query):
    return session.sql(f"SELECT _ockam_mssql_execute($${query}$$);").collect()[0][0]
';

-- Insert Statement
CREATE OR REPLACE FUNCTION ockam_mssql_insert(QUERY STRING, ENTRIES ARRAY)
    RETURNS VARCHAR
    CALLED ON NULL INPUT
    VOLATILE
    SERVICE = MSSQL_API_CLIENT
    ENDPOINT = 'http-endpoint'
    AS '/insert';
