GRANT CREATE COMPUTE POOL ON ACCOUNT TO APPLICATION kafka_ingest;
GRANT BIND SERVICE ENDPOINT ON ACCOUNT TO APPLICATION kafka_ingest;
GRANT USAGE ON WAREHOUSE ki_warehouse TO APPLICATION kafka_ingest;

-- start the application
CALL kafka_ingest.functions.restart_application(ARRAY_CONSTRUCT());
