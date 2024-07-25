USE on_role;
--GRANT CREATE COMPUTE POOL ON ACCOUNT TO APPLICATION ockam_node;
--GRANT BIND SERVICE ENDPOINT ON ACCOUNT TO APPLICATION ockam_node;
--GRANT USAGE ON WAREHOUSE on_warehouse TO APPLICATION ockam_node;
--
---- start the application
--CALL ockam_node.external.start_service(ARRAY_CONSTRUCT());
