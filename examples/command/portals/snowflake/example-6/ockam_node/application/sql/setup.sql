CREATE APPLICATION ROLE IF NOT EXISTS on_user;

CREATE SCHEMA IF NOT EXISTS internal;
GRANT USAGE ON SCHEMA internal TO APPLICATION ROLE on_user;

CREATE OR ALTER VERSIONED SCHEMA external;
GRANT USAGE ON SCHEMA external TO APPLICATION ROLE on_user;

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
                  image: /ockam_database/ockam_schema/ockam_repository/postgres_client:on
                  env:
                    POSTGRES_HOST: ${host}
                    POSTGRES_PORT: ${port}
                    POSTGRES_USER: ${postgresUser}
          $$
       EXTERNAL_ACCESS_INTEGRATIONS = (reference(\'ockam_external_access\'));`
    }).execute();

    return `Service postgres_client started on compute pool ${poolName}. Host: \'${host}\', Port: \'${port}\', User: \'${postgresUser}\'`;
';
GRANT USAGE ON procedure external.start_postgres_client(STRING, STRING, STRING) TO APPLICATION ROLE on_user;

CREATE OR REPLACE PROCEDURE external.start_ockam_node_service(NODE_CONFIGURATION STRING)
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
    var configuration = NODE_CONFIGURATION;
    snowflake.createStatement({ sqlText:
       `CREATE SERVICE IF NOT EXISTS internal.ockam_node
          IN COMPUTE POOL ${poolName}
          FROM SPECIFICATION $$
            spec:
              container:
                - name: ockam-node
                  image: /ockam_database/ockam_schema/ockam_repository/ockam_node:on
                  args:
                    - node
                    - create
                    - -vvv
                    - --foreground
                    - --configuration
                    - "${configuration}"
              endpoint:
              - name: ockam-endpoint
                port: 5433
                protocol: TCP
          $$
       EXTERNAL_ACCESS_INTEGRATIONS = (reference(\'ockam_external_access\'));`
    }).execute();

    return `Service ockam_node started on compute pool ${poolName} and configuration \'${configuration}\'`;
';
GRANT USAGE ON procedure external.start_ockam_node_service(STRING) TO APPLICATION ROLE on_user;

CREATE OR REPLACE PROCEDURE external.start_tcp_inlet(ENROLLMENT_TICKET STRING, RELAY STRING, ALLOW STRING)
    RETURNS STRING
    LANGUAGE JAVASCRIPT
    AS
'
    enrollmentTicket = ENROLLMENT_TICKET;
    viaRelay = RELAY;
    allowed = ALLOW;
    configuration = `\'{ node: ockam-inlet, tcp-listener-address: 0.0.0.0:0, ticket: ${enrollmentTicket}, tcp-inlet: { from: 0.0.0.0:5433, via: ${viaRelay}, allow: ${allowed} } }\'`;

    snowflake.createStatement({ sqlText: `CALL external.start_ockam_node_service(${configuration})`}).execute();
    return `Started an Ockam TCP inlet with configuration ${configuration}`;
';
GRANT USAGE ON procedure external.start_tcp_inlet(STRING, STRING, STRING) TO APPLICATION ROLE on_user;

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
     BEGIN
       CASE (ref_name)
         WHEN 'CONSUMER_EXTERNAL_ACCESS' THEN
           RETURN '{
             "type": "CONFIGURATION",
             "payload":{
               "host_ports": [
                 "k8s-hub-nginxing-7c763c63c5-12b7f3bf9ab0746a.elb.us-west-1.amazonaws.com:4015"
                 ],
               "allowed_secrets" : "NONE"
             }
           }';
       END CASE;
       RETURN 'unknow reference type';
     END;
  $$;

GRANT USAGE ON PROCEDURE external.get_external_access(STRING)
  TO APPLICATION ROLE on_user;

CREATE OR REPLACE PROCEDURE external.get_endpoint_url()
    RETURNS STRING
    LANGUAGE SQL
    AS
$$
DECLARE
    ingress_url STRING;
BEGIN
    SHOW ENDPOINTS IN SERVICE internal.ockam_node;
    SELECT "ingress_url" INTO :ingress_url FROM TABLE (RESULT_SCAN (LAST_QUERY_ID())) LIMIT 1;
    RETURN ingress_url;
END
$$;
GRANT USAGE ON PROCEDURE external.get_endpoint_url() TO APPLICATION ROLE on_user;

EXECUTE IMMEDIATE FROM 'support.sql';
