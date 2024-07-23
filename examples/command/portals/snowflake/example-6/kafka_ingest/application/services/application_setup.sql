CREATE APPLICATION ROLE IF NOT EXISTS ki_user;

CREATE SCHEMA IF NOT EXISTS core;
GRANT USAGE ON SCHEMA core TO APPLICATION ROLE ki_user;

CREATE OR ALTER VERSIONED SCHEMA functions;
GRANT USAGE ON SCHEMA functions TO APPLICATION ROLE ki_user;

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

   CREATE SERVICE IF NOT EXISTS core.kafka_ingest
     IN COMPUTE POOL identifier(:pool_name)
     FROM spec='services/spec.yml';

   RETURN 'Service successfully created with compute pool ' || pool_name;
END;
$$;

GRANT USAGE ON PROCEDURE functions.start_application() TO APPLICATION ROLE ki_user;

CREATE OR REPLACE PROCEDURE functions.stop_application()
    RETURNS string
    LANGUAGE sql
    AS
$$
BEGIN
    ALTER SERVICE IF EXISTS core.kafka_ingest FROM SPECIFICATION_FILE='services/spec.yml';
END
$$;
GRANT USAGE ON PROCEDURE functions.stop_application() TO APPLICATION ROLE ki_user;

CREATE OR REPLACE PROCEDURE functions.service_status()
RETURNS VARCHAR
LANGUAGE SQL
EXECUTE AS OWNER
AS $$
   DECLARE
         service_status VARCHAR;
   BEGIN
         SYSTEM$LOG('DEBUG', 'Calling the service status');
         CALL SYSTEM$GET_SERVICE_STATUS('core.kafka_ingest') INTO :service_status;
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
       CALL SYSTEM$GET_SERVICE_STATUS('core.kafka_ingest') INTO :service_status;
       RETURN :service_status;
   END;
$$;

GRANT USAGE ON PROCEDURE functions.service_status_all() TO APPLICATION ROLE ki_user;

CREATE OR REPLACE PROCEDURE functions.service_logs_kafka_consumer()
RETURNS VARCHAR
LANGUAGE SQL
EXECUTE AS OWNER
AS $$
   DECLARE
         service_logs VARCHAR;
   BEGIN
       SYSTEM$LOG('DEBUG', 'Calling the kafka ingest logs');
       CALL SYSTEM$GET_SERVICE_LOGS('core.kafka_ingest', '0', 'kafka-consumer', 1000) INTO :service_logs;
       RETURN :service_logs;
   END;
$$;

GRANT USAGE ON PROCEDURE functions.service_logs_kafka_consumer() TO APPLICATION ROLE ki_user;

CREATE OR REPLACE PROCEDURE functions.service_logs_ockam_kafka_inlet()
RETURNS VARCHAR
LANGUAGE SQL
EXECUTE AS OWNER
AS $$
   DECLARE
         service_logs VARCHAR;
   BEGIN
     SYSTEM$LOG('DEBUG', 'Calling the ockam kafka inlet logs');
     CALL SYSTEM$GET_SERVICE_LOGS('core.kafka_ingest', '0', 'ockam-kafka-inlet', 1000) INTO :service_logs;
     RETURN :service_logs;
   END;
$$;

GRANT USAGE ON PROCEDURE functions.service_logs_ockam_kafka_inlet() TO APPLICATION ROLE ki_user;

CREATE OR REPLACE PROCEDURE functions.register_reference(ref_name STRING, ref_or_alias STRING)
  RETURNS STRING
  LANGUAGE SQL
  AS $$
     BEGIN
        CALL SYSTEM$SET_REFERENCE(:ref_name, :ref_or_alias);
        SYSTEM$LOG('INFO', 'Stored the reference: ' || :ref_or_alias || ' with name ' || :ref_name);
        RETURN :ref_name;
    END;
  $$;

GRANT USAGE ON PROCEDURE functions.register_reference(STRING, STRING)
  TO APPLICATION ROLE ki_user;

CREATE OR REPLACE PROCEDURE functions.get_external_access(ref_name STRING)
  RETURNS STRING
  LANGUAGE SQL
  AS $$
     BEGIN
       CASE (ref_name)
         WHEN 'CONSUMER_EXTERNAL_ACCESS' THEN
           RETURN '{
             "type": "CONFIGURATION",
             "payload":{
               "host_ports": [
                 "k8s-hub-nginxing-7c763c63c5-12b7f3bf9ab0746a.elb.us-west-1.amazonaws.com:4015",
                 "k8s-hub-nginxing-7c763c63c5-12b7f3bf9ab0746a.elb.us-west-1.amazonaws.com:4015"
                 ],
               "allowed_secrets" : "NONE"
             }
           }';
       END CASE;
       RETURN 'unknow reference type';
     END;
  $$;

GRANT USAGE ON PROCEDURE functions.get_external_access(STRING)
  TO APPLICATION ROLE ki_user;
