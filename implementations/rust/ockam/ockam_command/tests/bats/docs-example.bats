#!/bin/bash

# ===== SETUP

setup_file() {
  load load/base.bash
}

setup() {
  load load/base.bash
  load load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data
}

#======== Kafka E2E constants
export OCKAM_HOME_CONSUMER=$(mktemp -d)
export OCKAM_HOME_PRODUCER_1=$(mktemp -d)
export OCKAM_HOME_PRODUCER_2=$(mktemp -d)
export KAFKA_CONFIG_FILE=/tmp/kafka.config
export CONSUMER_OUTPUT=/tmp/consumer.out
export CONSUMER_PID=/tmp/consumer.pid

#======== Basic web app constant
export FLASK_PID_FILE="/tmp/python.pid"
export FLASK_SERVER_FILE="/tmp/server.py"


teardown() {
  if ! cat /tmp/server.log; then
    touch /tmp/server.log
  fi

  echo $BATS_TEST_NAME >> /tmp/server.log
  if [[ $BATS_TEST_NAME == 'test_e2e_kafka' ]]; then
    echo "works for $BATS_TEST_NAME" >> /tmp/server.log
    kafka-topics.sh --bootstrap-server localhost:4000 --command-config $KAFKA_CONFIG_FILE --delete --topic demo-topic || echo ''

    if consumer_pid=$(cat $CONSUMER_PID); then
      kill $consumer_pid
    fi

    OCKAM_HOME="$OCKAM_HOME_CONSUMER" $OCKAM node delete --all --yes
    OCKAM_HOME="$OCKAM_HOME_PRODUCER_1" $OCKAM node delete --all --yes
    OCKAM_HOME="$OCKAM_HOME_PRODUCER_2" $OCKAM node delete --all --yes
  fi

  if [[ $BATS_TEST_NAME == 'test_basic_web_app' ]]; then
    echo "works for $BATS_TEST_NAME" >> /tmp/server.log
    if pid=$(cat "$FLASK_PID_FILE"); then
      kill -9 "$pid"
      wait "$pid" 2>/dev/null || true
    fi
  fi

  teardown_home_dir
}

# ====== TEST https://docs.ockam.io/guides/examples/basic-web-app
@test "basic web app" {
  if [[ -z $LOCAL_PG_HOST ]]; then
    export LOCAL_PG_HOST='127.0.0.1'
  fi

  ## We are listening to Ockam relay
  export OCKAM_PG_PORT=5433
  export LOCAL_PG_PORT=5432

  cat > $FLASK_SERVER_FILE <<- EOM
import os
import psycopg2
from flask import Flask
CREATE_TABLE = (
  "CREATE TABLE IF NOT EXISTS events (id SERIAL PRIMARY KEY, name TEXT);"
)
INSERT_RETURN_ID = "INSERT INTO events (name) VALUES (%s) RETURNING id;"
app = Flask(__name__)
url = "postgres://postgres:password@localhost/"
connection = psycopg2.connect(port=$OCKAM_PG_PORT, database="postgres", host="localhost", user="postgres", password="password")
@app.route("/")
def hello_world():
  with connection:
    with connection.cursor() as cursor:
        cursor.execute(CREATE_TABLE)
        cursor.execute(INSERT_RETURN_ID, ("",))
        id = cursor.fetchone()[0]
  return "I've been visited {} times".format(id), 201
if __name__ == "__main__":
  app.run(port=6000)
EOM

  ## Connecting the database
  export DB_TOKEN=$(ockam project ticket --attribute component=db)

  run_success $OCKAM identity create db
  run_success $OCKAM project enroll $DB_TOKEN --identity db
  run_success $OCKAM node create db --identity db
  run_success $OCKAM policy create --at db --resource tcp-outlet --expression '(= subject.component "web")'
  run_success $OCKAM tcp-outlet create --at /node/db --from /service/outlet --to $LOCAL_PG_HOST:$LOCAL_PG_PORT

  run_success $OCKAM relay create db --to /node/db --at /project/default

  ## Connecting the web app
  export WEB_TOKEN=$(ockam project ticket --attribute component=web)

  run_success $OCKAM identity create web
  run_success $OCKAM project enroll $WEB_TOKEN --identity web
  run_success $OCKAM node create web --identity web
  run_success $OCKAM policy create --at web --resource tcp-inlet --expression '(= subject.component "db")'
  run_success $OCKAM tcp-inlet create --at /node/web --from 127.0.0.1:$OCKAM_PG_PORT --to /project/default/service/forward_to_db/secure/api/service/outlet

  ## Kickstart webserver
  python3 $FLASK_SERVER_FILE &>>/tmp/server.log  &
  pid="$!"
  echo $pid > $FLASK_PID_FILE

  # Wait for server to kickstart
  sleep 5

  # Visit website
  run_success curl http://127.0.0.1:6000
  assert_output --partial "I've been visited 1 times"

  # Visit website second time
  run_success curl http://127.0.0.1:6000
  assert_output --partial "I've been visited 2 times"
}

# ===== TEST https://docs.ockam.io/guides/examples/end-to-end-encrypted-kafka
@test "e2e kafka" {
  if [[ -z $CONFLUENT_BOOTSTRAP_SERVER || -z $CONFLUENT_API_SECRET || -z $CONFLUENT_API_KEY ]]; then
    exit 1
  fi

  ## Configure the Confluent add-on
  run_success $OCKAM project addon configure confluent --bootstrap-server $CONFLUENT_BOOTSTRAP_SERVER

  $OCKAM project ticket --attribute role=member > /tmp/consumer.token
  $OCKAM project ticket --attribute role=member > /tmp/producer1.token
  $OCKAM project ticket --attribute role=member > /tmp/producer2.token



cat > $KAFKA_CONFIG_FILE <<EOF
request.timeout.ms=30000
security.protocol=SASL_PLAINTEXT
sasl.mechanism=PLAIN
sasl.jaas.config=org.apache.kafka.common.security.plain.PlainLoginModule required \
        username="$CONFLUENT_API_KEY" \
        password="$CONFLUENT_API_SECRET";
EOF

  export CURRENT_OCKAM_HOME=$OCKAM_HOME

  ## Consumer
  export OCKAM_HOME=$OCKAM_HOME_CONSUMER
  run_success $OCKAM identity create consumer
  run_success $OCKAM project enroll /tmp/consumer.token --identity consumer

  run_success $OCKAM node create consumer --identity consumer
  run_success $OCKAM kafka-consumer create --at consumer

  kafka-topics.sh --bootstrap-server localhost:4000 --command-config $KAFKA_CONFIG_FILE \
    --create --topic demo-topic --partitions 3

  kafka-console-consumer.sh --topic demo-topic \
    --bootstrap-server localhost:4000 --consumer.config $KAFKA_CONFIG_FILE > $CONSUMER_OUTPUT 2>&1 &

  consumer_pid="$!"
  echo "$consumer_pid" > $CONSUMER_PID

  ## Producer 1
  export OCKAM_HOME=$OCKAM_HOME_PRODUCER_1
  run_success $OCKAM identity create producer1
  run_success $OCKAM project enroll /tmp/producer1.token --identity producer1
  run_success $OCKAM node create producer1 --identity producer1

  run bash -c "$OCKAM kafka-producer create --at producer1 --bootstrap-server 127.0.0.1:6000" --brokers-port-range 6001-6100
  assert_success

  run bash -c "echo 'Hello from producer 1' | kafka-console-producer.sh --topic demo-topic\
    --bootstrap-server localhost:6000 --producer.config $KAFKA_CONFIG_FILE"
  assert_success

  run_success cat $CONSUMER_OUTPUT
  assert_output "Hello from producer 1"

  ## Producer 2
  export OCKAM_HOME=$OCKAM_HOME_PRODUCER_2
  run_success $OCKAM identity create producer2
  run_success $OCKAM project enroll /tmp/producer2.token --identity producer2
  run_success $OCKAM node create producer2 --identity producer2

  run_success $OCKAM kafka-producer create --at producer2 --bootstrap-server 127.0.0.1:7000 --brokers-port-range 7001-7100

  run bash -c "echo 'Hello from producer 2' | kafka-console-producer.sh \
    --topic demo-topic --bootstrap-server localhost:7000 --producer.config $KAFKA_CONFIG_FILE"
  assert_success

  run_success "cat $CONSUMER_OUTPUT"
  assert_output --partial "Hello from producer 2"

  export OCKAM_HOME=$CURRENT_OCKAM_HOME
}

#====== TEST https://docs.ockam.io/guides/examples/create-secure-communication-with-a-private-database-from-anywhere
@test "secure communication with private database" {
  run_success createdb app_db
  run_success "$OCKAM node create relay"
  run_success "$OCKAM node create db_sidecar"

  run_success "$OCKAM tcp-outlet create --at /node/db_sidecar --from /service/outlet --to 127.0.0.1:5432"
  run_success "$OCKAM relay create db_sidecar --at /node/relay --to /node/db_sidecar"

  run_success "$OCKAM node create client_sidecar"

  "$OCKAM" secure-channel create --from /node/client_sidecar --to /node/relay/service/forward_to_db_sidecar/service/api \
    | "$OCKAM" tcp-inlet create --at /node/client_sidecar --from 127.0.0.1:7777 --to -/service/outlet

  run_success psql --host='127.0.0.1' --port=7777 app_db
}
