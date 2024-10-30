function start_kafka() {
  docker compose --file "${BATS_TEST_DIRNAME}/${KAFKA_COMPOSE_FILE}" up --detach
  sleep 10
}

function stop_kafka() {
  docker compose --file "${BATS_TEST_DIRNAME}/${KAFKA_COMPOSE_FILE}" down
}

setup() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  stop_kafka
  teardown_home_dir
}

kafka_docker_end_to_end_encrypted_explicit_consumer() {
  # Admin
  export ADMIN_HOME="$OCKAM_HOME"

  export OCKAM_LOGGING=0
  outlet_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay kafka_outlet)
  consumer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay kafka_consumer)
  producer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member)

  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=info

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"
  export DIRECT_CONSUMER_OUTPUT="$ADMIN_HOME/direct_consumer.log"
  export KAFKA_CONFIG="$ADMIN_HOME/kafka.config"

  # Outlet
  setup_home_dir
  run_success "$OCKAM" project enroll "${outlet_ticket}"
  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" relay create kafka_outlet

  # Consumer
  setup_home_dir
  run_success "$OCKAM" project enroll "${consumer_ticket}"
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --avoid-publishing \
    --to /project/default/service/forward_to_kafka_outlet/secure/api
  run_success "$OCKAM" relay create kafka_consumer

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1
  kafka-console-consumer --topic demo --bootstrap-server localhost:29092 --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT" &
  # direct consumer
  kafka-console-consumer --topic demo --bootstrap-server localhost:19092 --max-messages 1 --timeout-ms 30000 >"$DIRECT_CONSUMER_OUTPUT" &

  # Producer
  setup_home_dir
  run_success "$OCKAM" project enroll "${producer_ticket}"
  run_success "$OCKAM" kafka-inlet create --from 39092 \
    --to /project/default/service/forward_to_kafka_outlet/secure/api \
    --consumer /project/default/service/forward_to_kafka_consumer/secure/api

  sleep 5
  run bash -c "echo 'Hello from producer' | kafka-console-producer --topic demo --bootstrap-server localhost:39092 --max-block-ms 30000"
  sleep 5

  run cat "$CONSUMER_OUTPUT"
  assert_output "Hello from producer"

  # direct connection to the kafka broker
  run cat "$DIRECT_CONSUMER_OUTPUT"
  refute_output --partial "Hello"
}

@test "kafka - docker - end-to-end-encrypted - explicit consumer - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_explicit_consumer
}

@test "kafka - docker - end-to-end-encrypted - explicit consumer - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_explicit_consumer
}

kafka_docker_end_to_end_encrypted_project_relay_consumer() {
  # Admin
  export ADMIN_HOME="$OCKAM_HOME"

  export OCKAM_LOGGING=0
  outlet_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay kafka_outlet)
  consumer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay '*')
  producer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member)

  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=info

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"
  export DIRECT_CONSUMER_OUTPUT="$ADMIN_HOME/direct_consumer.log"
  export KAFKA_CONFIG="$ADMIN_HOME/kafka.config"

  # Outlet
  setup_home_dir
  run_success "$OCKAM" project enroll "${outlet_ticket}"
  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" relay create kafka_outlet

  # Consumer
  setup_home_dir
  run_success "$OCKAM" project enroll "${consumer_ticket}"
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --publishing-relay /project/default \
    --to /project/default/service/forward_to_kafka_outlet/secure/api

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1
  kafka-console-consumer --topic demo --bootstrap-server localhost:29092 --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT" &
  # direct consumer
  kafka-console-consumer --topic demo --bootstrap-server localhost:19092 --max-messages 1 --timeout-ms 30000 >"$DIRECT_CONSUMER_OUTPUT" &

  # Producer
  setup_home_dir
  run_success "$OCKAM" project enroll "${producer_ticket}"
  run_success "$OCKAM" kafka-inlet create --from 39092 \
    --to /project/default/service/forward_to_kafka_outlet/secure/api \
    --consumer-relay /project/default

  sleep 5
  run bash -c "echo 'Hello from producer' | kafka-console-producer --topic demo --bootstrap-server localhost:39092 --max-block-ms 30000"
  sleep 5

  run cat "$CONSUMER_OUTPUT"
  assert_output "Hello from producer"

  # direct connection to the kafka broker
  run cat "$DIRECT_CONSUMER_OUTPUT"
  refute_output --partial "Hello"
}

@test "kafka - docker - end-to-end-encrypted - project relay consumer - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_project_relay_consumer
}

@test "kafka - docker - end-to-end-encrypted - project relay consumer - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_project_relay_consumer
}

kafka_docker_end_to_end_encrypted_rust_relay_consumer() {
  # Admin
  export ADMIN_HOME="$OCKAM_HOME"

  export OCKAM_LOGGING=0
  outlet_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay kafka_outlet)
  consumer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay '*')
  producer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member)

  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=info

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"
  export DIRECT_CONSUMER_OUTPUT="$ADMIN_HOME/direct_consumer.log"
  export KAFKA_CONFIG="$ADMIN_HOME/kafka.config"

  # Outlet
  setup_home_dir
  run_success "$OCKAM" project enroll "${outlet_ticket}"
  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" relay create kafka_outlet

  # Consumer
  setup_home_dir
  run_success "$OCKAM" project enroll "${consumer_ticket}"
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --publishing-relay /project/default/service/forward_to_kafka_outlet/secure/api \
    --to /project/default/service/forward_to_kafka_outlet/secure/api

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1
  kafka-console-consumer --topic demo --bootstrap-server localhost:29092 --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT" &
  # direct consumer
  kafka-console-consumer --topic demo --bootstrap-server localhost:19092 --max-messages 1 --timeout-ms 30000 >"$DIRECT_CONSUMER_OUTPUT" &

  # Producer
  setup_home_dir
  run_success "$OCKAM" project enroll "${producer_ticket}"
  run_success "$OCKAM" kafka-inlet create --from 39092 \
    --to /project/default/service/forward_to_kafka_outlet/secure/api \
    --consumer-relay /project/default/service/forward_to_kafka_outlet/secure/api

  sleep 5
  run bash -c "echo 'Hello from producer' | kafka-console-producer --topic demo --bootstrap-server localhost:39092 --max-block-ms 30000"
  sleep 5

  run cat "$CONSUMER_OUTPUT"
  assert_output "Hello from producer"

  # direct connection to the kafka broker
  run cat "$DIRECT_CONSUMER_OUTPUT"
  refute_output --partial "Hello"
}

@test "kafka - docker - end-to-end-encrypted - rust relay consumer - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_rust_relay_consumer
}

@test "kafka - docker - end-to-end-encrypted - rust relay consumer - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_rust_relay_consumer
}

kafka_docker_end_to_end_encrypted_direct_connection() {
  # Admin
  export ADMIN_HOME="$OCKAM_HOME"

  export OCKAM_LOGGING=0
  consumer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay kafka_consumer)
  producer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member)

  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=trace

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"
  export DIRECT_CONSUMER_OUTPUT="$ADMIN_HOME/direct_consumer.log"

  # Consumer
  setup_home_dir
  run_success "$OCKAM" project enroll "${consumer_ticket}"
  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --avoid-publishing \
    --to self
  run_success "$OCKAM" relay create kafka_consumer

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1
  kafka-console-consumer --topic demo --bootstrap-server localhost:29092 --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT" &
  # direct consumer
  kafka-console-consumer --topic demo --bootstrap-server localhost:19092 --max-messages 1 --timeout-ms 30000 >"$DIRECT_CONSUMER_OUTPUT" &

  # Producer
  setup_home_dir
  run_success "$OCKAM" project enroll "${producer_ticket}"
  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" kafka-inlet create --from 39092 \
    --to self \
    --consumer /project/default/service/forward_to_kafka_consumer/secure/api

  sleep 5
  run bash -c "echo 'Hello from producer' | kafka-console-producer --topic demo --bootstrap-server localhost:39092 --max-block-ms 30000"
  sleep 5

  run cat "$CONSUMER_OUTPUT"
  assert_output "Hello from producer"

  # direct connection to the kafka broker
  run cat "$DIRECT_CONSUMER_OUTPUT"
  refute_output --partial "Hello"
}

@test "kafka - docker - end-to-end-encrypted - direct connection - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_direct_connection
}

@test "kafka - docker - end-to-end-encrypted - direct connection - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_direct_connection
}

kafka_docker_end_to_end_encrypted_multiple_consumers_direct_connection() {
  inlet_port="$(random_port)"

  # Admin
  export ADMIN_HOME="$OCKAM_HOME"

  export OCKAM_LOGGING=0
  vault_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay vault)
  consumer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay kafka_consumer)
  producer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member)

  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=debug
  export RUST_BACKTRACE=full

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"
  export DIRECT_CONSUMER_OUTPUT="$ADMIN_HOME/direct_consumer.log"

  # Vault Node
  setup_home_dir
  run_success "$OCKAM" node create vault --tcp-listener-address 127.0.0.1:${inlet_port}
  run_success "$OCKAM" service start remote-proxy-vault --vault-name default

  # Consumer 1
  setup_home_dir
  # create a remote identity, but we need an enrolled local identity first
  run_success "$OCKAM" identity create local-identity
  run_success "$OCKAM" project enroll --identity local-identity "${consumer_ticket}"
  run_success "$OCKAM" message send --to /ip4/127.0.0.1/tcp/${inlet_port}/secure/api/service/echo hello
  assert_output "hello"
  run_success "$OCKAM" vault create \
    --route /ip4/127.0.0.1/tcp/${inlet_port}/secure/api/service/remote_proxy_vault \
    --identity local-identity remote-vault
  run_success "$OCKAM" identity create remote-identity --vault remote-vault
  run_success "$OCKAM" project enroll --identity remote-identity "${consumer_ticket}"
  run_success "$OCKAM" node create consumer --identity remote-identity

  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --avoid-publishing \
    --to self
  run_success "$OCKAM" relay create kafka_consumer

  # Consumer 2
  setup_home_dir
  # create a remote identity, but we need an enrolled local identity first
  run_success "$OCKAM" identity create local-identity
  run_success "$OCKAM" project enroll --identity local-identity "${consumer_ticket}"
  run_success "$OCKAM" message send --to /ip4/127.0.0.1/tcp/${inlet_port}/secure/api/service/echo hello
  assert_output "hello"
  run_success "$OCKAM" vault create \
    --route /ip4/127.0.0.1/tcp/${inlet_port}/secure/api/service/remote_proxy_vault \
    --identity local-identity remote-vault
  run_success "$OCKAM" identity create remote-identity --vault remote-vault
  run_success "$OCKAM" project enroll --identity remote-identity "${consumer_ticket}"
  run_success "$OCKAM" node create consumer --identity remote-identity

  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" kafka-inlet create --from 39092 \
    --avoid-publishing \
    --to self
  run_success "$OCKAM" relay create kafka_consumer

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1

  # Read messages, but just 2 from each consumer
  kafka-console-consumer --topic demo --bootstrap-server localhost:29092 --group consumer-group-name --max-messages 2 --timeout-ms 60000 >>"${CONSUMER_OUTPUT}" &
  kafka-console-consumer --topic demo --bootstrap-server localhost:39092 --group consumer-group-name --max-messages 2 --timeout-ms 60000 >>"${CONSUMER_OUTPUT}" &

  # direct consumer
  kafka-console-consumer --topic demo --bootstrap-server localhost:19092 --max-messages 1 --timeout-ms 60000 >"$DIRECT_CONSUMER_OUTPUT" &

  # Producer
  setup_home_dir
  run_success "$OCKAM" node create producer
  run_success "$OCKAM" project enroll "${producer_ticket}"
  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" kafka-inlet create --from 49092 \
    --to self \
    --consumer /project/default/service/forward_to_kafka_consumer/secure/api

  sleep 5
  run bash -c "echo 'Hello from producer - 1' | kafka-console-producer --topic demo --bootstrap-server localhost:49092 --max-block-ms 30000"
  run bash -c "echo 'Hello from producer - 2' | kafka-console-producer --topic demo --bootstrap-server localhost:49092 --max-block-ms 30000"
  run bash -c "echo 'Hello from producer - 3' | kafka-console-producer --topic demo --bootstrap-server localhost:49092 --max-block-ms 30000"
  run bash -c "echo 'Hello from producer - 4' | kafka-console-producer --topic demo --bootstrap-server localhost:49092 --max-block-ms 30000"
  sleep 20

  run cat "$CONSUMER_OUTPUT"
  assert_output --partial "Hello from producer - 1"
  assert_output --partial "Hello from producer - 2"
  assert_output --partial "Hello from producer - 3"
  assert_output --partial "Hello from producer - 4"

  # direct connection to the kafka broker
  run cat "$DIRECT_CONSUMER_OUTPUT"
  refute_output --partial "Hello"
}

@test "kafka - docker - end-to-end-encrypted - multiple consumers - direct connection - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_multiple_consumers_direct_connection
}

@test "kafka - docker - end-to-end-encrypted - multiple consumers - direct connection - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_multiple_consumers_direct_connection
}

kafka_docker_end_to_end_encrypted_single_gateway() {
  # Admin
  export ADMIN_HOME="$OCKAM_HOME"
  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=debug

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"
  export DIRECT_CONSUMER_OUTPUT="$ADMIN_HOME/direct_consumer.log"

  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --avoid-publishing \
    --to self \
    --consumer self

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1
  kafka-console-consumer --topic demo --bootstrap-server localhost:29092 --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT" &
  # direct consumer
  kafka-console-consumer --topic demo --bootstrap-server localhost:19092 --max-messages 1 --timeout-ms 30000 >"$DIRECT_CONSUMER_OUTPUT" &

  sleep 5
  run bash -c "echo 'Hello from producer' | kafka-console-producer --topic demo --bootstrap-server localhost:29092 --max-block-ms 30000"
  sleep 5

  run cat "$CONSUMER_OUTPUT"
  assert_output "Hello from producer"

  # direct connection to the kafka broker
  run cat "$DIRECT_CONSUMER_OUTPUT"
  refute_output --partial "Hello"
}

@test "kafka - docker - end-to-end-encrypted - single gateway - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_direct_connection
}

@test "kafka - docker - end-to-end-encrypted - single gateway - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_direct_connection
}

kafka_docker_cleartext() {
  # Admin
  export ADMIN_HOME="$OCKAM_HOME"

  export OCKAM_LOGGING=0
  outlet_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member --relay kafka_outlet)
  consumer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member)
  producer_ticket=$($OCKAM project ticket --usage-count 10 --attribute role=member)

  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=info

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"
  export DIRECT_CONSUMER_OUTPUT="$ADMIN_HOME/direct_consumer.log"
  export CONSUMER_OUTPUT_DIRECT="$ADMIN_HOME/consumer_direct.log"
  export KAFKA_CONFIG="$ADMIN_HOME/kafka.config"

  # Outlet
  setup_home_dir
  run_success "$OCKAM" project enroll "${outlet_ticket}"
  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" relay create kafka_outlet

  # Consumer
  setup_home_dir
  run_success "$OCKAM" project enroll "${consumer_ticket}"
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --avoid-publishing \
    --disable-content-encryption \
    --to /project/default/service/forward_to_kafka_outlet/secure/api

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1
  kafka-console-consumer --topic demo --bootstrap-server localhost:29092 --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT" &
  # direct consumer
  kafka-console-consumer --topic demo --bootstrap-server localhost:19092 --max-messages 1 --timeout-ms 30000 >"$DIRECT_CONSUMER_OUTPUT" &

  # Producer
  setup_home_dir
  run_success "$OCKAM" project enroll "${producer_ticket}"
  run_success "$OCKAM" kafka-inlet create --from 39092 \
    --disable-content-encryption \
    --to /project/default/service/forward_to_kafka_outlet/secure/api

  sleep 5
  run bash -c "echo 'Hello from producer' | kafka-console-producer --topic demo --bootstrap-server localhost:39092 --max-block-ms 30000"
  sleep 5

  run cat "$CONSUMER_OUTPUT"
  assert_output "Hello from producer"

  # direct connection to the kafka broker
  run cat "$DIRECT_CONSUMER_OUTPUT"
  assert_output "Hello from producer"
}

@test "kafka - docker - cleartext - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_cleartext
}

@test "kafka - docker - cleartext - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_cleartext
}

kafka_docker_end_to_end_encrypted_offset_decryption() {
  # Admin
  export ADMIN_HOME="$OCKAM_HOME"
  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=info

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"

  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --disable-content-encryption \
    --avoid-publishing \
    --to self \
    --consumer self

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1

  sleep 5
  run bash -c 'for n in {0..100}; do echo "message n. $n"; done | kafka-console-producer --topic demo --bootstrap-server localhost:29092 --max-block-ms 30000'
  sleep 5

  kafka-console-consumer --topic demo \
    --bootstrap-server localhost:29092 \
    --partition 0 \
    --offset 0 \
    --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT"

  run cat "$CONSUMER_OUTPUT"
  assert_output "message n. 0"

  kafka-console-consumer --topic demo \
    --bootstrap-server localhost:29092 \
    --partition 0 \
    --offset 100 \
    --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT"

  run cat "$CONSUMER_OUTPUT"
  assert_output "message n. 100"

  kafka-console-consumer --topic demo \
    --bootstrap-server localhost:29092 \
    --partition 0 \
    --offset 37 \
    --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT"

  run cat "$CONSUMER_OUTPUT"
  assert_output "message n. 37"
}

@test "kafka - docker - end-to-end-encrypted - offset decryption - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_offset_decryption
}

@test "kafka - docker - end-to-end-encrypted - offset decryption - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_end_to_end_encrypted_offset_decryption
}

kafka_docker_encrypt_only_two_fields() {
  # Admin
  export ADMIN_HOME="$OCKAM_HOME"
  export OCKAM_LOGGING=true
  export OCKAM_LOG_LEVEL=info

  export CONSUMER_OUTPUT="$ADMIN_HOME/consumer.log"

  # create a kafka outlet and inlet with direct connection to the kafka instance
  run_success "$OCKAM" kafka-outlet create --bootstrap-server 127.0.0.1:19092
  run_success "$OCKAM" kafka-inlet create --from 29092 \
    --encrypted-field encrypted_field_one \
    --encrypted-field encrypted_field_two \
    --avoid-publishing \
    --to self \
    --consumer self

  run kafka-topics --bootstrap-server localhost:29092 --delete --topic demo || true
  sleep 5
  run_success kafka-topics --bootstrap-server localhost:29092 --create --topic demo --partitions 1 --replication-factor 1

  # we push different records in the same topic
  # ockam is expected to encrypt only the fields encrypted_field_one and encrypted_field_two
  sleep 5
  RECORDS=(
    '{"encrypted_field_one":"value1","encrypted_field_two":"value2","field_three":"value3"}'
    '{"encrypted_field_one":{"key": "value"},"encrypted_field_two":["hello","world"]}'
  )
  for record in "${RECORDS[@]}"; do echo $record; done | kafka-console-producer --topic demo --bootstrap-server localhost:29092 --max-block-ms 30000
  sleep 5

  # connect directly to the broker to get the "raw" records
  # the fields encrypted_field_one and encrypted_field_two should be encrypted
  kafka-console-consumer --topic demo \
    --bootstrap-server localhost:19092 \
    --partition 0 \
    --offset 0 \
    --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT"

  run bash -c "cat \"\$CONSUMER_OUTPUT\" | jq -r '.encrypted_field_one'"
  refute_output "value1"

  run bash -c "cat \"\$CONSUMER_OUTPUT\" | jq -r '.encrypted_field_two'"
  refute_output "value2"

  run bash -c "cat \"\$CONSUMER_OUTPUT\" | jq -r '.field_three'"
  assert_output "value3"

  # connect to the ockam kafka inlet to get the first record
  # the fields encrypted_field_one and encrypted_field_two should be decrypted
  kafka-console-consumer --topic demo \
    --bootstrap-server localhost:29092 \
    --partition 0 \
    --offset 0 \
    --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT"

  run bash -c "cat \"\$CONSUMER_OUTPUT\" | jq -r '.encrypted_field_one'"
  assert_output "value1"

  run bash -c "cat \"\$CONSUMER_OUTPUT\" | jq -r '.encrypted_field_two'"
  assert_output "value2"

  run bash -c "cat \"\$CONSUMER_OUTPUT\" | jq -r '.field_three'"
  assert_output "value3"

  # same, for the second record
  kafka-console-consumer --topic demo \
    --bootstrap-server localhost:29092 \
    --partition 0 \
    --offset 1 \
    --max-messages 1 --timeout-ms 30000 >"$CONSUMER_OUTPUT"

  run bash -c "cat \"\$CONSUMER_OUTPUT\" | jq -r '.encrypted_field_one.key'"
  assert_output "value"

  run bash -c "cat \"\$CONSUMER_OUTPUT\" | jq -r '.encrypted_field_two[0]'"
  assert_output "hello"
}

@test "kafka - docker - encrypt only two fields - redpanda" {
  export KAFKA_COMPOSE_FILE="redpanda-docker-compose.yaml"
  start_kafka
  kafka_docker_encrypt_only_two_fields
}

@test "kafka - docker - encrypt only two fields - apache" {
  export KAFKA_COMPOSE_FILE="apache-docker-compose.yaml"
  start_kafka
  kafka_docker_encrypt_only_two_fields
}
