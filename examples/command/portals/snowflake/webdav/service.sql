
USE ROLE webdav_role;
USE DATABASE webdav_database;
USE SCHEMA webdav_schema;
USE WAREHOUSE webdav_warehouse;

CREATE SERVICE webdav_service
  IN COMPUTE POOL webdav_compute_pool
  FROM SPECIFICATION $$
    spec:
      containers:
      - name: webdav
        image: /webdav_database/webdav_schema/webdav_image_repository/webdav
        env:
          ANONYMOUS_METHODS: ALL
        volumeMounts:
        - name: stage
          mountPath: /var/lib/dav/data
      - name: ockam
        image: /webdav_database/webdav_schema/webdav_image_repository/ockam
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
            relay: webdav-relay
            tcp-outlet:
              to: localhost:80
              allow: webdav-inlet
      volumes:
      - name: stage
        source: "@WEBDAV_STAGE"
      $$
   EXTERNAL_ACCESS_INTEGRATIONS = (webdav_ockam_egress_access_integration)
   MIN_INSTANCES=1
   MAX_INSTANCES=1;
