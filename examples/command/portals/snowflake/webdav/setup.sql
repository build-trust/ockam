
USE ROLE ACCOUNTADMIN;
CREATE ROLE IF NOT EXISTS webdav_role;
GRANT ROLE webdav_role TO ROLE ACCOUNTADMIN;

GRANT CREATE WAREHOUSE ON ACCOUNT TO ROLE webdav_role;
GRANT CREATE DATABASE ON ACCOUNT TO ROLE webdav_role;
GRANT CREATE COMPUTE POOL ON ACCOUNT TO ROLE webdav_role;
GRANT CREATE INTEGRATION ON ACCOUNT TO ROLE webdav_role;

USE ROLE webdav_role;

-----------------------------------------------------------------------------------------------------------------------
-- Create and use a warehouse

CREATE OR REPLACE WAREHOUSE webdav_warehouse WITH WAREHOUSE_SIZE='XSMALL';
USE WAREHOUSE webdav_warehouse;

-----------------------------------------------------------------------------------------------------------------------
-- Create a database, schema, and image repository

CREATE DATABASE IF NOT EXISTS webdav_database;
USE DATABASE webdav_database;

CREATE SCHEMA IF NOT EXISTS webdav_schema;
USE SCHEMA webdav_schema;

CREATE IMAGE REPOSITORY IF NOT EXISTS webdav_image_repository;
CREATE OR REPLACE STAGE webdav_stage ENCRYPTION = (type = 'SNOWFLAKE_SSE');

-----------------------------------------------------------------------------------------------------------------------
-- Create compute pool

CREATE COMPUTE POOL webdav_compute_pool
    MIN_NODES = 1
    MAX_NODES = 5
    INSTANCE_FAMILY = CPU_X64_XS;

-----------------------------------------------------------------------------------------------------------------------
-- Create network rule and external access integration

USE ROLE ACCOUNTADMIN;
GRANT CREATE NETWORK RULE ON SCHEMA webdav_database.webdav_schema TO ROLE webdav_role;

USE ROLE webdav_role;
USE DATABASE webdav_database;
USE WAREHOUSE webdav_warehouse;

CREATE OR REPLACE NETWORK RULE webdav_ockam_egress_access
    MODE = EGRESS
    TYPE = HOST_PORT
    VALUE_LIST = ('&{ egress_host_port }');

CREATE OR REPLACE EXTERNAL ACCESS INTEGRATION webdav_ockam_egress_access_integration
    ALLOWED_NETWORK_RULES = (webdav_ockam_egress_access)
    ENABLED = true;
