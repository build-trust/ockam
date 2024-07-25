CREATE OR REPLACE PROCEDURE external.service_status()
  RETURNS VARCHAR
  LANGUAGE SQL
  EXECUTE AS OWNER
  AS $$
     DECLARE
           service_status VARCHAR;
     BEGIN
           SYSTEM$LOG('DEBUG', 'Calling the service status');
           CALL SYSTEM$GET_SERVICE_STATUS('internal.ockam_node') INTO :service_status;
           RETURN PARSE_JSON(:service_status)[0]['status']::VARCHAR;
     END;
  $$;

GRANT USAGE ON PROCEDURE external.service_status() TO APPLICATION ROLE on_user;

CREATE OR REPLACE PROCEDURE external.service_status_full()
  RETURNS VARCHAR
  LANGUAGE SQL
  EXECUTE AS OWNER
  AS $$
     DECLARE
           service_status VARCHAR;
     BEGIN
         SYSTEM$LOG('DEBUG', 'Calling the service status all');
         CALL SYSTEM$GET_SERVICE_STATUS('internal.ockam_node') INTO :service_status;
         RETURN :service_status;
     END;
  $$;

GRANT USAGE ON PROCEDURE external.service_status_full() TO APPLICATION ROLE on_user;

CREATE OR REPLACE PROCEDURE external.service_logs()
  RETURNS VARCHAR
  LANGUAGE SQL
  EXECUTE AS OWNER
  AS $$
     DECLARE
           service_logs VARCHAR;
     BEGIN
       SYSTEM$LOG('DEBUG', 'Calling the ockam_node logs');
       CALL SYSTEM$GET_SERVICE_LOGS('internal.ockam_node', '0', 'ockam-node', 1000) INTO :service_logs;
       RETURN :service_logs;
     END;
  $$;

GRANT USAGE ON PROCEDURE external.service_logs() TO APPLICATION ROLE on_user;
