-- Create a role for running the application
-- and grant the relevant permissions to create a native application

CREATE ROLE cdc_role;
GRANT ROLE cdc_role TO USER $USER_NAME;

GRANT CREATE INTEGRATION ON ACCOUNT TO ROLE cdc_role;
GRANT CREATE WAREHOUSE ON ACCOUNT TO ROLE cdc_role;
GRANT CREATE DATABASE ON ACCOUNT TO ROLE cdc_role;
GRANT CREATE APPLICATION PACKAGE ON ACCOUNT TO ROLE cdc_role;
GRANT CREATE APPLICATION ON ACCOUNT TO ROLE cdc_role;
GRANT CREATE COMPUTE POOL ON ACCOUNT TO ROLE cdc_role WITH GRANT OPTION;
GRANT BIND SERVICE ENDPOINT ON ACCOUNT TO ROLE cdc_role WITH GRANT OPTION;

-- Create a warehouse and a database that will be used as the source of change events

USE ROLE cdc_role;

CREATE OR REPLACE WAREHOUSE cdc_warehouse WITH
  WAREHOUSE_SIZE = 'X-SMALL'
  AUTO_SUSPEND = 180
  AUTO_RESUME = true
  INITIALLY_SUSPENDED = false;

CREATE DATABASE IF NOT EXISTS cdc_database;
CREATE SCHEMA IF NOT EXISTS cdc_schema;

USE WAREHOUSE cdc_warehouse;
USE DATABASE cdc_database;
USE SCHEMA cdc_schema;

-- Create an image repository where the service images will be uploaded
CREATE IMAGE REPOSITORY cdc_repository;

-- Create 2 tables which will generate change events

CREATE OR REPLACE TABLE customers (
    name VARCHAR(256),
    age NUMBER
);
GRANT ALL ON TABLE customers TO ROLE cdc_role;
ALTER TABLE customers SET CHANGE_TRACKING=TRUE;

CREATE OR REPLACE TABLE orders (
    product_id VARCHAR(256),
    customer VARCHAR(256),
    price NUMBER
);
GRANT ALL ON TABLE orders TO ROLE cdc_role;
ALTER TABLE orders SET CHANGE_TRACKING=TRUE;

-- Create network rules and an integration endpoint to be able to communicate outside of Snowflake

CREATE OR REPLACE NETWORK RULE cdc_ocsp_out
TYPE = 'HOST_PORT' MODE= 'EGRESS'
VALUE_LIST = ('ocsp.snowflakecomputing.com:80');

-- Update VALUE_LIST with ockam egress details
CREATE OR REPLACE NETWORK RULE cdc_ockam_out TYPE = 'HOST_PORT' MODE = 'EGRESS'
VALUE_LIST = $EGRESS_ALLOW_LIST;

CREATE OR REPLACE EXTERNAL ACCESS INTEGRATION cdc_external_access
ALLOWED_NETWORK_RULES = (cdc_ocsp_out, cdc_ockam_out)
ENABLED = true;

-- Create a table for logs
USE ROLE CDC_ROLE;
CREATE EVENT TABLE cdc_database.cdc_schema.cdc_events;
GRANT ALL ON EVENT TABLE cdc_database.cdc_schema.cdc_events TO ROLE ACCOUNTADMIN;

USE ROLE ACCOUNTADMIN;
ALTER ACCOUNT SET EVENT_TABLE = cdc_database.cdc_schema.cdc_events;

SHOW PARAMETERS LIKE 'event_table' IN ACCOUNT;
GRANT MODIFY LOG LEVEL ON ACCOUNT TO ROLE cdc_role;
