CREATE APPLICATION ROLE IF NOT EXISTS postgres_user;

CREATE SCHEMA IF NOT EXISTS internal;
GRANT USAGE ON SCHEMA internal TO APPLICATION ROLE postgres_user;

CREATE OR ALTER VERSIONED SCHEMA external;
GRANT USAGE ON SCHEMA external TO APPLICATION ROLE postgres_user;

CREATE OR REPLACE PROCEDURE external.start_postgres_client(HOST STRING, PORT STRING, POSTGRES_USER STRING)
    RETURNS STRING
    LANGUAGE JAVASCRIPT
    AS
'
    result = snowflake.createStatement({ sqlText: `SELECT CURRENT_DATABASE()`}).execute();
    result.next()
    current_database = result.getColumnValue(1);
    poolName = current_database + `_compute_pool`;
    snowflake.createStatement({ sqlText:
      `CREATE COMPUTE POOL IF NOT EXISTS ${poolName}
           MIN_NODES = 1
           MAX_NODES = 1
           INSTANCE_FAMILY = CPU_X64_XS
           AUTO_RESUME = true;`
      }).execute();

    snowflake.createStatement({ sqlText: `DROP SERVICE IF EXISTS internal.postgres_client`}).execute();
    var host = HOST;
    var port = PORT;
    var postgresUser = POSTGRES_USER;
    snowflake.createStatement({ sqlText:
       `CREATE SERVICE IF NOT EXISTS internal.postgres_client
          IN COMPUTE POOL ${poolName}
          FROM SPECIFICATION $$
            spec:
              container:
                - name: postgres-client
                  image: /consumer_database/consumer_schema/consumer_repository/postgres_client
                  env:
                    POSTGRES_HOST: ${host}
                    POSTGRES_PORT: ${port}
                    POSTGRES_USER: ${postgresUser}
          $$
       EXTERNAL_ACCESS_INTEGRATIONS = (reference(\'ocsp_external_access\'));
       `
    }).execute();

    return `Service postgres_client started on compute pool ${poolName}. Host: \'${host}\', Port: \'${port}\', User: \'${postgresUser}\'`;
';
GRANT USAGE ON procedure external.start_postgres_client(STRING, STRING, STRING) TO APPLICATION ROLE postgres_user;


CREATE OR REPLACE PROCEDURE external.register_reference(ref_name STRING, operation STRING, ref_or_alias STRING)
  RETURNS STRING
  LANGUAGE SQL
  AS $$
    BEGIN
      CASE (operation)
        WHEN 'ADD' THEN
          SELECT SYSTEM$SET_REFERENCE(:ref_name, :ref_or_alias);
        WHEN 'REMOVE' THEN
          SELECT SYSTEM$REMOVE_REFERENCE(:ref_name, :ref_or_alias);
        WHEN 'CLEAR' THEN
          SELECT SYSTEM$REMOVE_ALL_REFERENCES(:ref_name);
      ELSE
        RETURN 'unknown operation: ' || operation;
      END CASE;
      RETURN ref_or_alias;
    END;
  $$;

GRANT USAGE ON PROCEDURE external.register_reference(STRING, STRING, STRING)
  TO APPLICATION ROLE postgres_user;

CREATE OR REPLACE PROCEDURE external.get_external_access(ref_name STRING)
  RETURNS STRING
  LANGUAGE SQL
  AS $$
     BEGIN
       CASE (ref_name)
         WHEN 'OCSP_EXTERNAL_ACCESS' THEN
           RETURN '{
             "type": "CONFIGURATION",
             "payload": {
               "host_ports": [ "ocsp.snowflakecomputing.com:80" ],
               "allowed_secrets" : "NONE"
             }
           }';
       END CASE;
       RETURN '{"error": "unknown reference type: ' || ref_name || '"}';
     END;
  $$;

GRANT USAGE ON PROCEDURE external.get_external_access(STRING) TO APPLICATION ROLE postgres_user;

EXECUTE IMMEDIATE FROM 'support.sql';
