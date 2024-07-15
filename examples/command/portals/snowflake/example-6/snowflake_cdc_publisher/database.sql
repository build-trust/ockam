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

CREATE DATABASE cdc_image_database;
CREATE SCHEMA cdc_image_schema;
CREATE IMAGE REPOSITORY cdc_image_repository;
