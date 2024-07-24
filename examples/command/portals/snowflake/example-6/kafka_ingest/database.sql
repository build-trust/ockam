-- Create a role for running the application
-- and grant the relevant permissions to create a native application

CREATE ROLE ki_role;
GRANT ROLE ki_role TO USER $USER_NAME;

GRANT CREATE INTEGRATION ON ACCOUNT TO ROLE ki_role;
GRANT CREATE WAREHOUSE ON ACCOUNT TO ROLE ki_role;
GRANT CREATE DATABASE ON ACCOUNT TO ROLE ki_role;
GRANT CREATE APPLICATION PACKAGE ON ACCOUNT TO ROLE ki_role;
GRANT CREATE APPLICATION ON ACCOUNT TO ROLE ki_role;
GRANT CREATE COMPUTE POOL ON ACCOUNT TO ROLE ki_role WITH GRANT OPTION;
GRANT BIND SERVICE ENDPOINT ON ACCOUNT TO ROLE ki_role WITH GRANT OPTION;
GRANT MANAGE GRANTS ON ACCOUNT TO ROLE ki_role;

-- Create a warehouse and a database that will receive the kafka events

USE ROLE ki_role;

CREATE OR REPLACE WAREHOUSE ki_warehouse WITH
  WAREHOUSE_SIZE = 'X-SMALL'
  AUTO_SUSPEND = 180
  AUTO_RESUME = true
  INITIALLY_SUSPENDED = false;

CREATE DATABASE IF NOT EXISTS ki_database;
CREATE SCHEMA IF NOT EXISTS ki_schema;

USE WAREHOUSE ki_warehouse;
USE DATABASE ki_database;
USE SCHEMA ki_schema;

-- Create an image repository where the service images will be uploaded
CREATE IMAGE REPOSITORY ki_repository;

-- Create a table that will accept Kafka events coming from the customers topic

CREATE OR REPLACE TABLE customers (
    name VARCHAR(256),
    age NUMBER
);
GRANT ALL ON TABLE customers TO ROLE ki_role;

-- Create network rules and an integration endpoint to be able to communicate outside of Snowflake

CREATE OR REPLACE NETWORK RULE ki_ocsp_out
TYPE = 'HOST_PORT' MODE= 'EGRESS'
VALUE_LIST = ('ocsp.snowflakecomputing.com:80');

CREATE OR REPLACE NETWORK RULE ki_ockam_out TYPE = 'HOST_PORT' MODE = 'EGRESS'
VALUE_LIST = ('k8s-hub-nginxing-7c763c63c5-12b7f3bf9ab0746a.elb.us-west-1.amazonaws.com:4015','k8s-hub-nginxing-7c763c63c5-12b7f3bf9ab0746a.elb.us-west-1.amazonaws.com:4015');

CREATE OR REPLACE EXTERNAL ACCESS INTEGRATION ki_external_access
ALLOWED_NETWORK_RULES = (ki_ocsp_out, ki_ockam_out)
ENABLED = true;

-- Create a table for logs
USE ROLE ki_role;
CREATE EVENT TABLE ki_database.ki_schema.ki_events;
GRANT ALL ON EVENT TABLE ki_database.ki_schema.ki_events TO ROLE ACCOUNTADMIN;

USE ROLE ACCOUNTADMIN;
ALTER ACCOUNT SET EVENT_TABLE = ki_database.ki_schema.ki_events;

SHOW PARAMETERS LIKE 'event_table' IN ACCOUNT;
GRANT MODIFY LOG LEVEL ON ACCOUNT TO ROLE ki_role;
