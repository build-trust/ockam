
USE ROLE ACCOUNTADMIN;
CREATE ROLE IF NOT EXISTS sftp_role;
GRANT ROLE sftp_role TO ROLE ACCOUNTADMIN;

GRANT CREATE WAREHOUSE ON ACCOUNT TO ROLE sftp_role;
GRANT CREATE DATABASE ON ACCOUNT TO ROLE sftp_role;
GRANT CREATE COMPUTE POOL ON ACCOUNT TO ROLE sftp_role;
GRANT CREATE INTEGRATION ON ACCOUNT TO ROLE sftp_role;

USE ROLE sftp_role;

-----------------------------------------------------------------------------------------------------------------------
-- Create and use a warehouse

CREATE OR REPLACE WAREHOUSE sftp_warehouse WITH WAREHOUSE_SIZE='XSMALL';
USE WAREHOUSE sftp_warehouse;

-----------------------------------------------------------------------------------------------------------------------
-- Create a database, schema, and image repository

CREATE DATABASE IF NOT EXISTS sftp_database;
USE DATABASE sftp_database;

CREATE SCHEMA IF NOT EXISTS sftp_schema;
USE SCHEMA sftp_schema;

CREATE IMAGE REPOSITORY IF NOT EXISTS sftp_image_repository;
CREATE OR REPLACE STAGE sftp_stage ENCRYPTION = (type = 'SNOWFLAKE_SSE');

-----------------------------------------------------------------------------------------------------------------------
-- Create compute pool

CREATE COMPUTE POOL sftp_compute_pool
    MIN_NODES = 1
    MAX_NODES = 5
    INSTANCE_FAMILY = CPU_X64_XS;

-----------------------------------------------------------------------------------------------------------------------
-- Create network rule and external access integration

USE ROLE ACCOUNTADMIN;
GRANT CREATE NETWORK RULE ON SCHEMA sftp_database.sftp_schema TO ROLE sftp_role;

USE ROLE sftp_role;
USE DATABASE sftp_database;
USE WAREHOUSE sftp_warehouse;

CREATE OR REPLACE NETWORK RULE sftp_ockam_egress_access
    MODE = EGRESS
    TYPE = HOST_PORT
    VALUE_LIST = ('&{ egress_host_port }');

CREATE OR REPLACE EXTERNAL ACCESS INTEGRATION sftp_ockam_egress_access_integration
    ALLOWED_NETWORK_RULES = (sftp_ockam_egress_access)
    ENABLED = true;
