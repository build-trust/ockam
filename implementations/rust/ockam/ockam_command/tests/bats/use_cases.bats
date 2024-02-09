#!/bin/bash

# https://docs.ockam.io/use-cases

# ===== SETUP
setup_file() {
  load load/base.bash
}

setup() {
  load load/base.bash
  load load/orchestrator.bash
  load load/docs.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  kill_kafka_contents || true
  kill_flask_server || true
  kill_telegraf_instance || true
  teardown_home_dir
}

# ===== TESTS

# https://docs.ockam.io/guides/use-cases/add-end-to-end-encryption-to-any-client-and-server-application-with-no-code-change
# Please update the docs repository if this bats test is updated
@test "use-case - end-to-end encryption, local" {
  port="$(random_port)"
  run_success "$OCKAM" node create relay

  # Service
  run_success "$OCKAM" node create server_sidecar

  run_success "$OCKAM" tcp-outlet create --at /node/server_sidecar --to 127.0.0.1:5000
  run_success "$OCKAM" relay create server_sidecar --at /node/relay --to /node/server_sidecar
  assert_output --partial "forward_to_server_sidecar"

  # Client
  run_success "$OCKAM" node create client_sidecar
  run_success bash -c "$OCKAM secure-channel create --from /node/client_sidecar --to /node/relay/service/forward_to_server_sidecar/service/api \
              | $OCKAM tcp-inlet create --at /node/client_sidecar --from 127.0.0.1:$port --to -/service/outlet"

  run_success curl --head --max-time 10 "127.0.0.1:$port"
}

# https://docs.ockam.io/
# Please update the docs repository if this bats test is updated
@test "use-case - end-to-end encryption, orchestrator" {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data

  inlet_port="$(random_port)"

  # Service
  run_success "$OCKAM" tcp-outlet create --to 5000
  run_success "$OCKAM" relay create

  # Client
  run_success $OCKAM tcp-inlet create --from "$inlet_port"

  run_success curl --head --max-time 10 "127.0.0.1:$inlet_port"
}

# https://docs.ockam.io/use-cases/apply-fine-grained-permissions-with-attribute-based-access-control-abac
# Please update the docs repository if this bats test is updated
@test "use-case - abac" {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data

  port_1=9002
  port_2=9003
  fwd=$(random_str)

  # Administrator
  ADMIN_HOME="$OCKAM_HOME"
  run_success bash -c "$OCKAM project ticket --attribute component=control --relay $fwd > $OCKAM_HOME/control.ticket"
  run_success bash -c "$OCKAM project ticket --attribute component=edge > $OCKAM_HOME/edge.ticket"
  run_success bash -c "$OCKAM project ticket --attribute component=x > $OCKAM_HOME/x.ticket"

  # Control plane
  setup_home_dir
  run_success "$OCKAM" project import --project-file $PROJECT_PATH

  run_success $OCKAM identity create control_identity
  run_success $OCKAM project enroll "$ADMIN_HOME/control.ticket" --identity control_identity
  run_success $OCKAM node create control_plane1 --identity control_identity
  run_success $OCKAM tcp-outlet create --at /node/control_plane1 \
    --to 127.0.0.1:5000 --policy '(= subject.component "edge")'
  run_success $OCKAM relay create "$fwd" --at /project/default --to /node/control_plane1

  # Edge plane
  setup_home_dir
  run_success "$OCKAM" project import --project-file $PROJECT_PATH

  $OCKAM identity create edge_identity
  $OCKAM project enroll "$ADMIN_HOME/edge.ticket" --identity edge_identity
  $OCKAM node create edge_plane1 --identity edge_identity
  $OCKAM tcp-inlet create --at /node/edge_plane1 --from "127.0.0.1:$port_1" \
    --to "$fwd" --policy '(= subject.component "control")'
  run_success curl --fail --head --max-time 10 "127.0.0.1:$port_1"

  ## The following is denied
  $OCKAM identity create x_identity
  $OCKAM project enroll "$ADMIN_HOME/x.ticket" --identity x_identity
  $OCKAM node create x --identity x_identity
  $OCKAM tcp-inlet create --at /node/x --from "127.0.0.1:$port_2" \
    --to "$fwd" --policy '(= subject.component "control")'
  run curl --fail --head --max-time 10 "127.0.0.1:$port_2"
  assert_failure 28 # timeout error
}

# https://docs.ockam.io/guides/examples/basic-web-app
# Please update the docs repository if this bats test is updated
@test "use-case - basic-web-app" {
  skip_if_docs_tests_not_enabled
  copy_local_orchestrator_data

  export OCKAM_PG_PORT=5433
  export OCKAM_PG_PORT_MACHINE_C=5434
  export PG_PORT=5432
  export FLASK_PORT="$(random_port)"

  MACHINE_A="$OCKAM_HOME"
  run_success $OCKAM tcp-outlet create --to "$PG_HOST:$PG_PORT"
  run_success $OCKAM relay create

  run_success $OCKAM tcp-inlet create --from $OCKAM_PG_PORT

  # Kickstart webserver
  export APP_PG_PORT="$OCKAM_PG_PORT"
  run_success start_python_server

  # Visit website
  run_success curl "http://127.0.0.1:$FLASK_PORT"
  assert_output --partial "I've been visited 1 times"
  # Visit website second time
  run_success curl "http://127.0.0.1:$FLASK_PORT"
  assert_output --partial "I've been visited 2 times"

  run_success kill_flask_server

  # Run the web app on different machines

  # On machine A
  run_success bash -c "$OCKAM project ticket --attribute component=db --relay db > ${MACHINE_A}/db.ticket"
  run_success bash -c "$OCKAM project ticket --attribute component=web > ${MACHINE_A}/webapp.ticket"

  # Machine B
  setup_home_dir
  run_success $OCKAM identity create db
  run_success $OCKAM project enroll "${MACHINE_A}/db.ticket" --identity db
  run_success $OCKAM node create db --identity db
  run_success $OCKAM tcp-outlet create --to "$PG_HOST:$PG_PORT" --policy '(= subject.component "web")'
  run_success $OCKAM relay create db

  # Machine C
  setup_home_dir
  run_success $OCKAM identity create web
  run_success $OCKAM project enroll ${MACHINE_A}/webapp.ticket --identity web
  run_success $OCKAM node create web --identity web
  run_success $OCKAM tcp-inlet create --from "$OCKAM_PG_PORT_MACHINE_C" --to db --policy '(= subject.component "db")'

  export APP_PG_PORT="$OCKAM_PG_PORT_MACHINE_C"
  run_success start_python_server

  # Visit website
  run_success curl "http://127.0.0.1:$FLASK_PORT"
  assert_output --partial "I've been visited 3 times"
  # Visit website second time
  run_success curl "http://127.0.0.1:$FLASK_PORT"
  assert_output --partial "I've been visited 4 times"
}

# https://docs.ockam.io/guides/examples/create-secure-communication-with-a-private-database-from-anywhere
# Please update the docs repository if this bats test is updated
@test "use-case - create-secure-communication-with-a-private-database-from-anywhere" {
  skip_if_docs_tests_not_enabled
  copy_local_orchestrator_data

  export PGHOST="$PG_HOST"
  export PGPASSWORD="password"
  run_success createdb -U postgres app_db

  run_success "$OCKAM" tcp-outlet create --to "$PG_HOST:5432"
  run_success "$OCKAM" relay create

  run_success $OCKAM tcp-inlet create --from 7777
  # Call the list database -l
  run_success psql --host="127.0.0.1" --port=7777 -U postgres app_db -l
}

# https://docs.ockam.io/guides/examples/okta
# Please update the docs repository if this bats test is updated
@test "use-case - okta" {
  skip "not yet finalized" # We require an okta login we performing ockam enroll --okta, enrolling automatically isn't supported right now
  skip_if_docs_tests_not_enabled
  copy_local_orchestrator_data

  ADMIN_HOME="$OCKAM_HOME"
  run_success "$OCKAM" project addon configure okta \
    --tenant "$OKTA_TENANT" --client-id "$OKTA_CLIENT_ID" \
    --attribute email --attribute city --attribute department

  run_success bash -c "$OCKAM project information --output json > project.json"

  # Generate enrollment tickets
  run_success bash -c "$OCKAM project ticket --attribute application='Smart Factory' --attribute city='San Francisco' --relay m1 > m1.ticket"
  run_success bash -c "$OCKAM project ticket --attribute application='Smart Factory' --attribute city='New York' --relay m2 > m2.ticket"

  # Machine 1
  setup_home_dir
  run_success "$OCKAM" identity create m1
  run_success "$OCKAM" project enroll m1.ticket --identity m1
  run_success "$OCKAM" node create m1 --identity m1
  run_success "$OCKAM" tcp-outlet create --at /node/m1 --from /service/outlet --to 127.0.0.1:5000 \
    --policy '(or (= subject.application "Smart Factory") (and (= subject.department "Field Engineering") (= subject.city "San Francisco")))'
  run_success "$OCKAM" relay create m1 --at /project/default --to /node/m1

  # Machine 2
  setup_home_dir
  run_success "$OCKAM" identity create m2
  run_success "$OCKAM" project enroll m2.ticket --identity m2
  run_success "$OCKAM" node create m2 --identity m2
  run_success "$OCKAM" tcp-outlet create --at /node/m2 --from /service/outlet --to 127.0.0.1:6000 \
    --policy '(or (= subject.application "Smart Factory") (and (= subject.department "Field Engineering") (= subject.city "New York")))'
  run_success "$OCKAM" relay create m2 --at /project/default --to /node/m2

  # Alice
  setup_home_dir
  run_success "$OCKAM" project import --project-file project.json
  run_success "$OCKAM" project enroll --okta
  run_success "$OCKAM" node create alice
  run_success "$OCKAM" policy create --at alice --resource tcp-inlet

  # Alice request to access Machine 1 in San Francisco is allowed
  run_success "$OCKAM" tcp-inlet create --at /node/alice --from 127.0.0.1:8000 --to m1 --policy '(= subject.application "Smart Factory")'
  run_success curl --head 127.0.0.1:8000

  # Alice request to access Machine 2 in New York is denied
  run_success "$OCKAM" tcp-inlet create --at /node/alice --from 127.0.0.1:9000 --to m2 --policy '(= subject.application "Smart Factory")'
  run_failure curl --head 127.0.0.1:9000
}

# https://docs.ockam.io/guides/examples/end-to-end-encrypted-kafka
# Please update the docs repository if this bats test is updated
@test "use-case - end-to-end-encrypted-kafka" {
  skip_if_docs_tests_not_enabled
  copy_local_orchestrator_data

  # Admin
  export ADMIN_HOME="$OCKAM_HOME"
  run_success "$OCKAM" project addon configure confluent --bootstrap-server "$CONFLUENT_CLOUD_BOOTSTRAP_SERVER_ADDRESS"
  run_success bash -c "$OCKAM project ticket --attribute role=member --relay '*' > ${ADMIN_HOME}/consumer.ticket"
  run_success bash -c "$OCKAM project ticket --attribute role=member --relay '*' > ${ADMIN_HOME}/producer1.ticket"
  run_success bash -c "$OCKAM project ticket --attribute role=member --relay '*' > ${ADMIN_HOME}/producer2.ticket"

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"
  export KAFKA_CONFIG="$ADMIN_HOME/kafka.config"

  cat >"$KAFKA_CONFIG" <<EOF
request.timeout.ms=30000
security.protocol=SASL_PLAINTEXT
sasl.mechanism=PLAIN
sasl.jaas.config=org.apache.kafka.common.security.plain.PlainLoginModule required \
        username="$CONFLUENT_CLOUD_KAFKA_CLUSTER_API_KEY" \
        password="$CONFLUENT_CLOUD_KAFKA_CLUSTER_API_SECRET";
EOF

  export DEMO_TOPIC="$(random_str)"

  # Consumer
  setup_home_dir
  run_success "$OCKAM" identity create consumer
  run_success "$OCKAM" project enroll "${ADMIN_HOME}/consumer.ticket" --identity consumer
  run_success "$OCKAM" node create consumer --identity consumer
  run_success "$OCKAM" kafka-consumer create --at consumer

  run kafka-topics.sh --bootstrap-server localhost:4000 --command-config "$KAFKA_CONFIG" --create --topic "$DEMO_TOPIC" --partitions 3
  kafka-console-consumer.sh --topic "$DEMO_TOPIC" \
    --bootstrap-server localhost:4000 --consumer.config "$KAFKA_CONFIG" >"$CONSUMER_OUTPUT" 2>&1 &

  consumer_pid="$!"
  echo "$consumer_pid" >"$ADMIN_HOME/kafka.pid"

  # Producer 1
  run_success "$OCKAM" identity create producer1
  run_success "$OCKAM" project enroll "${ADMIN_HOME}/producer1.ticket" --identity producer1
  run_success "$OCKAM" node create producer1 --identity producer1
  run_success "$OCKAM" kafka-producer create --at producer1 --bootstrap-server 127.0.0.1:6000

  run bash -c "echo 'Hello from producer 1' | kafka-console-producer.sh --topic $DEMO_TOPIC \
    --bootstrap-server localhost:6000 \
    --producer.config $KAFKA_CONFIG"

  run_success cat $CONSUMER_OUTPUT
  assert_output "Hello from producer 1"

  # Producer 2
  setup_home_dir
  run_success "$OCKAM" identity create producer2
  run_success "$OCKAM" project enroll "${ADMIN_HOME}/producer2.ticket" --identity producer2
  run_success "$OCKAM" node create producer2 --identity producer2

  run_success "$OCKAM" kafka-producer create --at producer2 \
    --bootstrap-server 127.0.0.1:7000

  run_success bash -c "echo 'Hello from producer 2' | kafka-console-producer.sh --topic $DEMO_TOPIC \
    --bootstrap-server localhost:7000 \
    --producer.config $KAFKA_CONFIG"

  run_success cat $CONSUMER_OUTPUT
  assert_output --partial "Hello from producer 2"
}

# https://docs.ockam.io/guides/examples/telegraf-+-influxdb
# Please update the docs repository if this bats test is updated
@test "use-case - Telegraf + InfluxDB" {
  skip_if_docs_tests_not_enabled
  copy_local_orchestrator_data

  export ADMIN_HOME="$OCKAM_HOME"
  run_success start_telegraf_instance

  # Ensure that telegraf works without using Ockam route
  run_success curl \
    --header "Authorization: Token $INFLUX_TOKEN" \
    --header "Accept: application/csv" \
    --header 'Content-type: application/vnd.flux' \
    --data "from(bucket:\"$INFLUX_BUCKET\") |> range(start:-1m)" \
    "http://localhost:$INFLUX_PORT/api/v2/query?org=$INFLUX_ORG"

  run_success bash -c "$OCKAM project ticket --attribute component=influxdb --relay influxdb > ${ADMIN_HOME}/influxdb.ticket"
  run_success bash -c "$OCKAM project ticket --attribute component=telegraf > ${ADMIN_HOME}/telegraf.ticket"

  # InfluxDB instance
  setup_home_dir
  run_success "$OCKAM" identity create influxdb
  ockam project enroll "${ADMIN_HOME}/influxdb.ticket" --identity influxdb
  run_success "$OCKAM" node create influxdb --identity influxdb
  run_success "$OCKAM" tcp-outlet create --at /node/influxdb --from /service/outlet \
    --to "127.0.0.1:${INFLUX_PORT}" --policy '(= subject.component "telegraf")'
  run_success "$OCKAM" relay create influxdb --at /project/default --to /node/influxdb

  # Telegraf instance
  setup_home_dir
  export INFLUX_PORT="$(random_port)"

  run_success "$OCKAM" identity create telegraf
  run_success "$OCKAM" project enroll "${ADMIN_HOME}/telegraf.ticket" --identity telegraf
  run_success "$OCKAM" node create telegraf --identity telegraf
  run_success "$OCKAM" tcp-inlet create --at /node/telegraf --from "127.0.0.1:${INFLUX_PORT}" \
    --to influxdb --policy '(= subject.component "influxdb")'

  run_success kill_telegraf_instance
  run_success start_telegraf_instance

  # Ensure that telegraf works with using Ockam route
  run_success curl \
    --header "Authorization: Token $INFLUX_TOKEN" \
    --header "Accept: application/csv" \
    --header 'Content-type: application/vnd.flux' \
    --data "from(bucket:\"$INFLUX_BUCKET\") |> range(start:-1m)" \
    "http://localhost:$INFLUX_PORT/api/v2/query?org=$INFLUX_ORG"
}

# https://docs.ockam.io/guides/examples/okta
# Please update the docs repository if this bats test is updated
@test "use-case - InfluxDB Cloud token lease management" {
  skip "Influx DB needs a fix" # Not working currently
  skip_if_docs_tests_not_enabled
  copy_local_orchestrator_data

  export INFLUXDB_LEASE_PERMISSIONS="[{\"action\":  \"read\", \"resource\": {\"type\": \"authorizations\", \"orgID\": \"$INFLUXDB_ORG_ID\"}}]"
  export ADMIN_HOME="$OCKAM_HOME"

  run_success "$OCKAM" project addon configure influxdb \
    --endpoint-url "$INFLUXDB_ENDPOINT_URL" \
    --token "$INFLUXDB_ADMIN_TOKEN" \
    --org-id "$INFLUXDB_ORG_ID" \
    --permissions "$INFLUXDB_LEASE_PERMISSIONS" \
    --max-ttl 900

  run_success bash -c "$OCKAM project ticket --attribute service=iot-sensor > ${ADMIN_HOME}/sensor.ticket"

  # Client
  setup_home_dir
  run_success "$OCKAM" identity create iot-sensor
  run_success "$OCKAM" project enroll "${ADMIN_HOME}/sensor.ticket" --identity iot-sensor
  run_success "$OCKAM" lease create --identity iot-sensor
}
