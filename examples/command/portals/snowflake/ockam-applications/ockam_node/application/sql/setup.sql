CREATE APPLICATION ROLE IF NOT EXISTS on_user;

CREATE SCHEMA IF NOT EXISTS internal;
GRANT USAGE ON SCHEMA internal TO APPLICATION ROLE on_user;

CREATE TABLE IF NOT EXISTS internal.ockam_project_url (url STRING) ;
GRANT ALL ON TABLE internal.ockam_project_url TO APPLICATION ROLE on_user;

CREATE OR ALTER VERSIONED SCHEMA external;
GRANT USAGE ON SCHEMA external TO APPLICATION ROLE on_user;

CREATE OR REPLACE PROCEDURE external.start_ockam_node_service(NODE_CONFIGURATION STRING, PORT STRING)
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

    snowflake.createStatement({ sqlText: `DROP SERVICE IF EXISTS internal.ockam_node`}).execute();
    configuration = NODE_CONFIGURATION;
    port = PORT;
    snowflake.createStatement({ sqlText:
       `CREATE SERVICE IF NOT EXISTS internal.ockam_node
          IN COMPUTE POOL ${poolName}
          FROM SPECIFICATION $$
            spec:
              container:
                - name: ockam-node
                  image: /ockam_database/ockam_schema/ockam_repository/ockam
                  env:
                    OCKAM_DISABLE_UPGRADE_CHECK: true
                    OCKAM_OPENTELEMETRY_EXPORT: false
                  args:
                    - node
                    - create
                    - -vv
                    - --foreground
                    - --configuration
                    - "${configuration}"
              endpoint:
              - name: ockam-endpoint
                port: ${port}
                protocol: TCP
          $$
       EXTERNAL_ACCESS_INTEGRATIONS = (reference(\'ockam_external_access\'));`
    }).execute();

    return `SUCCESS`;
';
GRANT USAGE ON procedure external.start_ockam_node_service(STRING, STRING) TO APPLICATION ROLE on_user;

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
  TO APPLICATION ROLE on_user;

CREATE OR REPLACE PROCEDURE external.get_external_access(ref_name STRING)
  RETURNS STRING
  LANGUAGE SQL
  AS $$
     DECLARE
       ockam_project_url STRING;
     BEGIN
       SELECT url INTO :ockam_project_url from internal.ockam_project_url;
       CASE (ref_name)
         WHEN 'OCKAM_EXTERNAL_ACCESS' THEN
           RETURN '{
             "type": "CONFIGURATION",
             "payload": {
               "host_ports": [ "' || :ockam_project_url || '" ],
               "allowed_secrets" : "NONE"
             }
           }';
       END CASE;
       RETURN '{"error": "unknown reference type: ' || ref_name || '"}';
     END;
  $$;

GRANT USAGE ON PROCEDURE external.get_external_access(STRING) TO APPLICATION ROLE on_user;

DROP STREAMLIT IF EXISTS internal.ui;

CREATE STREAMLIT IF NOT EXISTS internal.configuration
  FROM '/streamlit'
  MAIN_FILE = '/configuration.py'
;
GRANT USAGE ON STREAMLIT internal.configuration TO APPLICATION ROLE on_user;

EXECUTE IMMEDIATE FROM 'support.sql';
