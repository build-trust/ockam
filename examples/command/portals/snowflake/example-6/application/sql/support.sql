CREATE OR REPLACE PROCEDURE external.postgres_client_service_status()
  RETURNS VARCHAR
  LANGUAGE SQL
  EXECUTE AS OWNER
  AS $$
     DECLARE
           service_status VARCHAR;
     BEGIN
           SYSTEM$LOG('DEBUG', 'Calling the service status');
           CALL SYSTEM$GET_SERVICE_STATUS('internal.postgres_client') INTO :service_status;
           RETURN PARSE_JSON(:service_status)[0]['status']::VARCHAR;
     END;
  $$;

GRANT USAGE ON PROCEDURE external.postgres_client_service_status() TO APPLICATION ROLE postgres_user;

CREATE OR REPLACE PROCEDURE external.postgres_client_service_status_all()
  RETURNS VARCHAR
  LANGUAGE SQL
  EXECUTE AS OWNER
  AS $$
     DECLARE
           service_status VARCHAR;
     BEGIN
         SYSTEM$LOG('DEBUG', 'Calling the service status all');
         CALL SYSTEM$GET_SERVICE_STATUS('internal.postgres_client') INTO :service_status;
         RETURN :service_status;
     END;
  $$;

GRANT USAGE ON PROCEDURE external.postgres_client_service_status_all() TO APPLICATION ROLE postgres_user;

CREATE OR REPLACE PROCEDURE external.postgres_client_service_logs()
  RETURNS VARCHAR
  LANGUAGE SQL
  EXECUTE AS OWNER
  AS $$
     DECLARE
           service_logs VARCHAR;
     BEGIN
       SYSTEM$LOG('DEBUG', 'Calling the postgres_client logs');
       CALL SYSTEM$GET_SERVICE_LOGS('internal.postgres_client', '0', 'postgres-client', 1000) INTO :service_logs;
       RETURN :service_logs;
     END;
  $$;

GRANT USAGE ON PROCEDURE external.postgres_client_service_logs() TO APPLICATION ROLE postgres_user;
