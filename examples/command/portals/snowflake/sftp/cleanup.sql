USE ROLE ACCOUNTADMIN;

DROP SERVICE IF EXISTS sftp_database.sftp_schema.sftp_service;
DROP INTEGRATION IF EXISTS sftp_ockam_egress_access_integration;
DROP COMPUTE POOL IF EXISTS sftp_compute_pool;
DROP DATABASE IF EXISTS sftp_database;
DROP WAREHOUSE IF EXISTS sftp_warehouse;

DROP ROLE IF EXISTS sftp_role;
