GRANT CREATE COMPUTE POOL ON ACCOUNT TO APPLICATION cdc_publisher;
GRANT BIND SERVICE ENDPOINT ON ACCOUNT TO APPLICATION cdc_publisher;
GRANT USAGE ON WAREHOUSE cdc_warehouse TO APPLICATION cdc_publisher;

-- start the application
CALL cdc_publisher.functions.start_application();
