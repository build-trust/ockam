
USE ROLE sftp_role;
USE DATABASE sftp_database;
USE SCHEMA sftp_schema;
USE WAREHOUSE sftp_warehouse;

CREATE SERVICE sftp_service
  IN COMPUTE POOL sftp_compute_pool
  FROM SPECIFICATION $$
    spec:
      containers:
      - name: sftp
        image: /sftp_database/sftp_schema/sftp_image_repository/sftp
        args:
         - foo::1001
        volumeMounts:
         - name: stage
           mountPath: /home/foo/stage
      - name: ockam
        image: /sftp_database/sftp_schema/sftp_image_repository/ockam
        env:
          OCKAM_DISABLE_UPGRADE_CHECK: true
          OCKAM_OPENTELEMETRY_EXPORT: false
        args:
          - node
          - create
          - -vv
          - --foreground
          - --enrollment-ticket
          - '&{ ticket }'
          - --configuration
          - |
            relay: sftp-relay
            tcp-outlet:
              to: localhost:22
              allow: sftp-inlet
      volumes:
      - name: stage
        source: "@SFTP_STAGE"
      $$
   EXTERNAL_ACCESS_INTEGRATIONS = (sftp_ockam_egress_access_integration)
   MIN_INSTANCES=1
   MAX_INSTANCES=1;
