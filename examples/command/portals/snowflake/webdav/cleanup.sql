USE ROLE ACCOUNTADMIN;

DROP SERVICE IF EXISTS webdav_database.webdav_schema.webdav_service;
DROP INTEGRATION IF EXISTS webdav_ockam_egress_access_integration;
DROP COMPUTE POOL IF EXISTS webdav_compute_pool;
DROP DATABASE IF EXISTS webdav_database;
DROP WAREHOUSE IF EXISTS webdav_warehouse;

DROP ROLE IF EXISTS webdav_role;
