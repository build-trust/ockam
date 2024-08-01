-- Create a role for installing and testing the application
USE ROLE ACCOUNTADMIN;
CREATE ROLE IF NOT EXISTS consumer_role;
GRANT ROLE consumer_role TO USER $USER_NAME;

GRANT CREATE WAREHOUSE ON ACCOUNT TO ROLE consumer_role;
GRANT CREATE DATABASE ON ACCOUNT TO ROLE consumer_role;
GRANT CREATE APPLICATION ON ACCOUNT TO ROLE consumer_role;
GRANT CREATE APPLICATION PACKAGE ON ACCOUNT TO ROLE consumer_role;
GRANT CREATE INTEGRATION ON ACCOUNT TO ROLE consumer_role;
GRANT CREATE COMPUTE POOL ON ACCOUNT TO ROLE consumer_role WITH GRANT OPTION;
GRANT BIND SERVICE ENDPOINT ON ACCOUNT TO ROLE consumer_role WITH GRANT OPTION;
GRANT MANAGE GRANTS ON ACCOUNT TO ROLE consumer_role;

CREATE OR REPLACE WAREHOUSE consumer_warehouse WITH
  WAREHOUSE_SIZE = 'X-SMALL'
  AUTO_SUSPEND = 180
  AUTO_RESUME = true
  INITIALLY_SUSPENDED = false;
GRANT ALL ON WAREHOUSE consumer_warehouse to ROLE consumer_role;
USE WAREHOUSE consumer_warehouse;

CREATE DATABASE IF NOT EXISTS consumer_database;
GRANT ALL ON DATABASE consumer_database TO ROLE consumer_role;
USE DATABASE consumer_database;

CREATE SCHEMA IF NOT EXISTS consumer_schema;
GRANT ALL ON SCHEMA consumer_database.consumer_schema TO ROLE consumer_role;
USE SCHEMA consumer_database.consumer_schema;

CREATE IMAGE REPOSITORY IF NOT EXISTS consumer_repository;
GRANT READ ON IMAGE REPOSITORY consumer_repository TO ROLE consumer_role;
GRANT WRITE ON IMAGE REPOSITORY consumer_repository TO ROLE consumer_role;

USE ROLE consumer_role;

-- Create a network rule and an integration endpoint to be able to check the Snowflake certificate
-- when connecting to Snowflake

CREATE OR REPLACE NETWORK RULE consumer_ocsp_out
TYPE = 'HOST_PORT' MODE= 'EGRESS'
VALUE_LIST = ('ocsp.snowflakecomputing.com:80');

CREATE OR REPLACE EXTERNAL ACCESS INTEGRATION consumer_external_access
ALLOWED_NETWORK_RULES = (consumer_ocsp_out)
ENABLED = true;
