# End-to-End Encrypted Data Streams using Redpanda Selfhosted

![Architecture](diagram.png)


# Get started with Ockam

[Signup for Ockam](https://www.ockam.io/signup) and then run the following commands on your workstation:

```sh
# Install Ockam Command
curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash && source "$HOME/.ockam/env"

# Enroll with Ockam Orchestrator.
ockam enroll

# Create enrollment ticket for the node that will run inside container services.
ockam project ticket --usage-count 1 --expires-in 1h --attribute consumer --relay '*' --attribute kafka-inlet > consumer.ticket

# Print the egress allow list for your Ockam project.
ockam project show --jq .egress_allow_list

```

## Setup Redpanda

- Start Local Redpanda Server with ockam

```sh
pushd redpanda_server; ENROLLMENT_TICKET=$(ockam project ticket --usage-count 1 --expires-in 1h --relay redpanda --attribute kafka-outlet) docker compose up; popd
```

- View console at http://localhost:8080/

- Create a topic named `topic_A`

## Setup Snowflake

```sql
USE ROLE ACCOUNTADMIN;

--Create Role
CREATE ROLE DATASTREAMS_ROLE;
GRANT ROLE DATASTREAMS_ROLE TO ROLE ACCOUNTADMIN;

--Create Database
CREATE DATABASE IF NOT EXISTS DATASTREAMS_DB;
GRANT OWNERSHIP ON DATABASE DATASTREAMS_DB TO ROLE DATASTREAMS_ROLE COPY CURRENT GRANTS;

--Create Warehouse
CREATE OR REPLACE WAREHOUSE DATASTREAMS_WH WITH WAREHOUSE_SIZE='X-SMALL';
GRANT USAGE ON WAREHOUSE DATASTREAMS_WH TO ROLE DATASTREAMS_ROLE;

--Create compute pool
CREATE COMPUTE POOL DATASTREAMS_CP
  MIN_NODES = 1
  MAX_NODES = 5
  INSTANCE_FAMILY = CPU_X64_XS;

GRANT USAGE ON COMPUTE POOL DATASTREAMS_CP TO ROLE DATASTREAMS_ROLE;
GRANT MONITOR ON COMPUTE POOL DATASTREAMS_CP TO ROLE DATASTREAMS_ROLE;

--Wait till compute pool is in idle or ready state
DESCRIBE COMPUTE POOL DATASTREAMS_CP;

--Create schema

CREATE SCHEMA IF NOT EXISTS DATASTREAMS_SCHEMA;
GRANT ALL PRIVILEGES ON SCHEMA DATASTREAMS_SCHEMA TO ROLE DATASTREAMS_ROLE;

--Create Image Repository
CREATE IMAGE REPOSITORY IF NOT EXISTS DATASTREAMS_REPOSITORY;
GRANT READ ON IMAGE REPOSITORY DATASTREAMS_REPOSITORY TO ROLE DATASTREAMS_ROLE;
--Note repository_url value to be used to build and publish consumer image to snowflake
SHOW IMAGE REPOSITORIES;

```

- Create table

```sql
USE ROLE DATASTREAMS_ROLE;
USE DATABASE DATASTREAMS_DB;
USE WAREHOUSE DATASTREAMS_WH;

CREATE or REPLACE TABLE DATASTREAMS_DB.DATASTREAMS_SCHEMA.KAFKA_MESSAGES (
	ID INTEGER,
	MESSAGE VARCHAR(256),
	EMAIL VARCHAR(256)
);
```

## Build and push Redpanda Connect Consumer Image


```sh
cd redpanda_connect_consumer

# Use the repository_url
docker login <repository_url>
docker build --rm --platform linux/amd64 -t <repository_url>/rps_connect_consumer .
docker push <repository_url>/rps_connect_consumer

```

## Setup Consumer in Snowpark

```sql
USE ROLE DATASTREAMS_ROLE;
USE DATABASE DATASTREAMS_DB;
USE WAREHOUSE DATASTREAMS_WH;
USE SCHEMA DATASTREAMS_SCHEMA;

--Update VALUE_LIST with ockam egress details
CREATE OR REPLACE NETWORK RULE DATASTREAMS_OCKAM_OUT
TYPE = 'HOST_PORT' MODE= 'EGRESS'
VALUE_LIST = ("TODO:TODO","TODO:TODO");

CREATE OR REPLACE NETWORK RULE DATASTREAMS_OCSP_OUT
TYPE = 'HOST_PORT' MODE= 'EGRESS'
VALUE_LIST = ('ocsp.snowflakecomputing.com:80');

-- Create access integration

USE ROLE ACCOUNTADMIN;
GRANT CREATE INTEGRATION ON ACCOUNT TO ROLE DATASTREAMS_ROLE;

CREATE OR REPLACE EXTERNAL ACCESS INTEGRATION DATASTREAMS_OCKAM_REDPANDA
ALLOWED_NETWORK_RULES = (DATASTREAMS_OCKAM_OUT, DATASTREAMS_OCSP_OUT)
ENABLED = true;

GRANT USAGE ON INTEGRATION DATASTREAMS_OCKAM_REDPANDA TO ROLE DATASTREAMS_ROLE;

-- Create service
USE ROLE DATASTREAMS_ROLE;

DROP SERVICE IF EXISTS REDPANDA_SERVER_CONNECT_OCKAM;

CREATE SERVICE REDPANDA_SERVER_CONNECT_OCKAM
  IN COMPUTE POOL DATASTREAMS_CP
  FROM SPECIFICATION
$$
    spec:
      containers:
      - name: consumer
        image: /datastreams_db/datastreams_schema/datastreams_repository/rps_connect_consumer
        env:
          SNOWFLAKE_WAREHOUSE: DATASTREAMS_WH
          OCKAM_ENROLLMENT_TICKET: "TODO"
$$
EXTERNAL_ACCESS_INTEGRATIONS = (DATASTREAMS_OCKAM_REDPANDA)
MIN_INSTANCES=1
MAX_INSTANCES=1;

SHOW SERVICES;
SELECT SYSTEM$GET_SERVICE_STATUS('REDPANDA_SERVER_CONNECT_OCKAM');
DESCRIBE SERVICE REDPANDA_SERVER_CONNECT_OCKAM;
CALL SYSTEM$GET_SERVICE_LOGS('REDPANDA_SERVER_CONNECT_OCKAM', '0', 'consumer', 1000);


```

## Run producer

```sh
# Use docker to run a producer to produce messages into redpanda
cd redpanda_connect_producer

docker run --rm \
  -v "$(pwd)/producer.yaml:/producer.yaml:ro" \
  -e OCKAM_ENROLLMENT_TICKET="$(ockam project ticket --usage-count 1 --expires-in 10m --attribute producer --attribute kafka-inlet)"  \
  ghcr.io/build-trust/redpanda-connect --config /producer.yaml

```

```sql
-- See the consumed messages that were stored in a table
SELECT * FROM DATASTREAMS_DB.DATASTREAMS_SCHEMA.KAFKA_MESSAGES;
```

## Cleanup

- Exit (Ctrl+C) from producer docker container

- Local files

```sh
rm consumer.ticket
```

- Stop redpanda server

```sh
pushd redpanda_server; docker compose down --rmi all --remove-orphans; popd
```

- Clean snowflake objects

```sql
USE ROLE DATASTREAMS_ROLE;
USE DATABASE DATASTREAMS_DB;
USE WAREHOUSE DATASTREAMS_WH;
USE SCHEMA DATASTREAMS_SCHEMA;

DROP SERVICE REDPANDA_SERVER_CONNECT_OCKAM;

DROP NETWORK RULE IF EXISTS DATASTREAMS_OCKAM_OUT;
DROP NETWORK RULE IF EXISTS DATASTREAMS_OCSP_OUT;

USE ROLE ACCOUNTADMIN;

DROP INTEGRATION IF EXISTS DATASTREAMS_OCKAM_REDPANDA;
DROP COMPUTE POOL IF EXISTS DATASTREAMS_CP;
DROP SCHEMA IF EXISTS DATASTREAMS_SCHEMA;
DROP WAREHOUSE IF EXISTS DATASTREAMS_WH;
DROP DATABASE IF EXISTS DATASTREAMS_DB;
DROP ROLE IF EXISTS DATASTREAMS_ROLE;

```
