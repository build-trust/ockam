!set variable_substitution=true
!variables

USE ROLE MSSQL_CONNECTOR_ROLE;
USE DATABASE MSSQL_CONNECTOR_DB;
USE WAREHOUSE MSSQL_CONNECTOR_WH;
USE SCHEMA MSSQL_CONNECTOR_SCHEMA;

DROP SERVICE IF EXISTS MSSQL_CONNECTOR_CLIENT;

CREATE SERVICE MSSQL_CONNECTOR_CLIENT
  IN COMPUTE POOL MSSQL_CONNECTOR_CP
  FROM SPECIFICATION
$$
    spec:
      endpoints:
      - name: http-endpoint
        port: 8080
        public: false
        protocol: HTTP
      - name: ockam-inlet
        port: 1443
        public: false
        protocol: TCP
      containers:
      - name: ockam-inlet
        image: /mssql_connector_db/mssql_connector_schema/mssql_connector_repository/ockam
        env:
            OCKAM_DISABLE_UPGRADE_CHECK: true
            OCKAM_TELEMETRY_EXPORT: false
        args:
          - node
          - create
          - --foreground
          - --enrollment-ticket
          - "&ockam_ticket"
          - --configuration
          - |
            tcp-inlet:
              from: 0.0.0.0:1433
              via: mssql
              allow: mssql
      - name: http-endpoint
        image: /mssql_connector_db/mssql_connector_schema/mssql_connector_repository/mssql_client
        env:
          SNOWFLAKE_WAREHOUSE: MSSQL_CONNECTOR_WH
          MSSQL_DATABASE: '&mssql_database'
          MSSQL_USER: '&mssql_user'
          MSSQL_PASSWORD: '&mssql_password'
        resources:
          requests:
            cpu: 0.5
            memory: 128M
          limits:
            cpu: 1
            memory: 256M
$$
MIN_INSTANCES=1
MAX_INSTANCES=1
EXTERNAL_ACCESS_INTEGRATIONS = (OCSP, OCKAM);
