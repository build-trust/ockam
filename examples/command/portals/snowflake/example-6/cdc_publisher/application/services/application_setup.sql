CREATE APPLICATION ROLE IF NOT EXISTS cdc_user;

CREATE SCHEMA IF NOT EXISTS core;
GRANT USAGE ON SCHEMA core TO APPLICATION ROLE cdc_user;

CREATE OR ALTER VERSIONED SCHEMA functions;
GRANT USAGE ON SCHEMA functions TO APPLICATION ROLE cdc_user;

CREATE OR REPLACE PROCEDURE functions.start_application()
   RETURNS string
   LANGUAGE sql
   AS
$$
BEGIN
   -- This account-level compute pool object is prefixed with the database name to prevent clashes
   LET pool_name := (SELECT CURRENT_DATABASE()) || '_compute_pool';

   CREATE COMPUTE POOL IF NOT EXISTS IDENTIFIER(:pool_name)
      MIN_NODES = 1
      MAX_NODES = 1
      INSTANCE_FAMILY = CPU_X64_XS
      AUTO_RESUME = true;

   CREATE SERVICE IF NOT EXISTS core.cdc_publisher
      IN COMPUTE POOL identifier(:pool_name)
      FROM spec='services/spec.yml';

   RETURN 'Service successfully created with compute pool ' || pool_name;
END;
$$;

GRANT USAGE ON PROCEDURE functions.start_application() TO APPLICATION ROLE cdc_user;

CREATE OR REPLACE PROCEDURE functions.stop_application()
    RETURNS string
    LANGUAGE sql
    AS
$$
BEGIN
    ALTER SERVICE IF EXISTS core.cdc_publisher FROM SPECIFICATION_FILE='services/spec.yml';
END
$$;
GRANT USAGE ON PROCEDURE functions.stop_application() TO APPLICATION ROLE cdc_user;

CREATE OR REPLACE PROCEDURE functions.service_status()
RETURNS VARCHAR
LANGUAGE SQL
EXECUTE AS OWNER
AS $$
   DECLARE
         service_status VARCHAR;
   BEGIN
         SYSTEM$LOG('DEBUG', 'Calling the service status');
         CALL SYSTEM$GET_SERVICE_STATUS('core.cdc_publisher') INTO :service_status;
         RETURN PARSE_JSON(:service_status)[0]['status']::VARCHAR;
   END;
$$;

CREATE OR REPLACE PROCEDURE functions.service_status_all()
RETURNS VARCHAR
LANGUAGE SQL
EXECUTE AS OWNER
AS $$
   DECLARE
         service_status VARCHAR;
   BEGIN
       SYSTEM$LOG('DEBUG', 'Calling the service status all');
       CALL SYSTEM$GET_SERVICE_STATUS('core.cdc_publisher') INTO :service_status;
       RETURN :service_status;
   END;
$$;

GRANT USAGE ON PROCEDURE functions.service_status_all() TO APPLICATION ROLE cdc_user;

CREATE OR REPLACE PROCEDURE functions.service_logs_cdc_publisher()
RETURNS VARCHAR
LANGUAGE SQL
EXECUTE AS OWNER
AS $$
   DECLARE
         service_logs VARCHAR;
   BEGIN
       SYSTEM$LOG('DEBUG', 'Calling the cdc publisher logs');
       CALL SYSTEM$GET_SERVICE_LOGS('core.cdc_publisher', '0', 'cdc-publisher', 1000) INTO :service_logs;
       RETURN :service_logs;
   END;
$$;

GRANT USAGE ON PROCEDURE functions.service_logs_cdc_publisher() TO APPLICATION ROLE cdc_user;

CREATE OR REPLACE PROCEDURE functions.service_logs_ockam_inlet()
RETURNS VARCHAR
LANGUAGE SQL
EXECUTE AS OWNER
AS $$
   DECLARE
         service_logs VARCHAR;
   BEGIN
     SYSTEM$LOG('DEBUG', 'Calling the ockam inlet logs');
     CALL SYSTEM$GET_SERVICE_LOGS('core.cdc_publisher', '0', 'ockam-inlet', 1000) INTO :service_logs;
     RETURN :service_logs;
   END;
$$;

GRANT USAGE ON PROCEDURE functions.service_logs_ockam_inlet() TO APPLICATION ROLE cdc_user;

CREATE OR REPLACE PROCEDURE functions.register_table_to_stream(ref_name STRING, ref_or_alias STRING)
  RETURNS STRING
  LANGUAGE SQL
  AS $$
     DECLARE
         stream_name STRING;
         stream_schema STRING;
     BEGIN
        CALL SYSTEM$SET_REFERENCE(:ref_name, :ref_or_alias);
        CREATE OR REPLACE STREAM core.cdc_stream ON TABLE reference(:ref_name);
        SYSTEM$LOG('INFO', 'Created the stream core.cdc_stream on the table: ' || :ref_or_alias || ' with reference ' || :ref_name);
        RETURN 'core.cdc_stream';
    END;
  $$;

GRANT USAGE ON PROCEDURE functions.register_table_to_stream(STRING, STRING)
  TO APPLICATION ROLE cdc_user;
