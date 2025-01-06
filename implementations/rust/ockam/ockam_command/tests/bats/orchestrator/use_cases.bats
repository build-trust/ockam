#!/bin/bash

# https://docs.ockam.io/use-cases

# ===== SETUP

setup() {
  load ../load/base.bash
  load_bats_ext
  setup_home_dir
  load ../load/orchestrator.bash
  skip_if_orchestrator_tests_not_enabled
  load ../load/docs.bash
  skip_if_docs_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  kill_telegraf_instance || true
  teardown_home_dir
}

# ===== TESTS

# https://docs.ockam.io/
# Please update the docs repository if this bats test is updated
@test "use-case - end-to-end encryption, orchestrator" {
  inlet_port="$(random_port)"
  relay_name="$(random_str)"

  # Service
  run_success "$OCKAM" tcp-outlet create --to $PYTHON_SERVER_PORT
  run_success "$OCKAM" relay create $relay_name

  # Client
  run_success $OCKAM tcp-inlet create --from "$inlet_port" --via "$relay_name"
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"
}

# https://docs.ockam.io/use-cases/apply-fine-grained-permissions-with-attribute-based-access-control-abac
# Please update the docs repository if this bats test is updated
@test "use-case - abac" {
  port_1=$(random_port)
  port_2=$(random_port)
  relay_name=$(random_str)

  # Administrator
  ADMIN_HOME="$OCKAM_HOME"
  run_success bash -c "$OCKAM project ticket --usage-count 10 --attribute component=control --relay $relay_name > $OCKAM_HOME/control.ticket"
  run_success bash -c "$OCKAM project ticket --usage-count 10 --attribute component=edge > $OCKAM_HOME/edge.ticket"
  run_success bash -c "$OCKAM project ticket --usage-count 10 --attribute component=x > $OCKAM_HOME/x.ticket"

  # Control plane
  setup_home_dir
  run_success "$OCKAM" project import --project-file $PROJECT_PATH

  run_success $OCKAM identity create control_identity
  run_success $OCKAM project enroll "$ADMIN_HOME/control.ticket" --identity control_identity
  run_success $OCKAM node create control_plane1 --identity control_identity
  run_success $OCKAM tcp-outlet create --at /node/control_plane1 \
    --to 127.0.0.1:$PYTHON_SERVER_PORT --allow '(= subject.component "edge")'
  run_success $OCKAM relay create "$relay_name" --at /project/default --to /node/control_plane1

  # Edge plane
  setup_home_dir
  run_success "$OCKAM" project import --project-file $PROJECT_PATH

  $OCKAM identity create edge_identity
  $OCKAM project enroll "$ADMIN_HOME/edge.ticket" --identity edge_identity
  $OCKAM node create edge_plane1 --identity edge_identity
  $OCKAM tcp-inlet create --at /node/edge_plane1 --from "$port_1" \
    --via "$relay_name" --allow '(= subject.component "control")'
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port_1"

  ## The following is denied
  $OCKAM identity create x_identity
  $OCKAM project enroll "$ADMIN_HOME/x.ticket" --identity x_identity
  $OCKAM node create x --identity x_identity
  $OCKAM tcp-inlet create --at /node/x --from "$port_2" \
    --via "$relay_name" --allow '(= subject.component "control")'
  run curl -sfI -m 5 "127.0.0.1:$port_2"
  assert_failure 28 # timeout error
}

# https://docs.ockam.io/guides/examples/telegraf-+-influxdb
# Please update the docs repository if this bats test is updated
@test "use-case - Telegraf + InfluxDB" {
  export ADMIN_HOME="$OCKAM_HOME"
  run_success start_telegraf_instance
  relay_name=$(random_str)

  # Ensure that telegraf works without using Ockam route
  run_success curl -s \
    --retry-all-errors --retry-delay 5 --retry 10 \
    --header "Authorization: Token $INFLUX_TOKEN" \
    --header "Accept: application/csv" \
    --header 'Content-type: application/vnd.flux' \
    --data "from(bucket:\"$INFLUX_BUCKET\") |> range(start:-1m)" \
    "http://localhost:$INFLUX_PORT/api/v2/query?org=$INFLUX_ORG"

  run_success bash -c "$OCKAM project ticket --usage-count 10 --attribute component=influxdb --relay $relay_name > ${ADMIN_HOME}/influxdb.ticket"
  run_success bash -c "$OCKAM project ticket --usage-count 10 --attribute component=telegraf > ${ADMIN_HOME}/telegraf.ticket"

  # InfluxDB instance
  setup_home_dir
  run_success "$OCKAM" identity create influxdb
  ockam project enroll "${ADMIN_HOME}/influxdb.ticket" --identity influxdb
  run_success "$OCKAM" node create influxdb --identity influxdb
  run_success "$OCKAM" tcp-outlet create --at /node/influxdb \
    --to "127.0.0.1:${INFLUX_PORT}" --allow '(= subject.component "telegraf")'
  run_success "$OCKAM" relay create $relay_name --at /project/default --to /node/influxdb

  # Telegraf instance
  setup_home_dir
  export INFLUX_PORT="$(random_port)"

  run_success "$OCKAM" identity create telegraf
  run_success "$OCKAM" project enroll "${ADMIN_HOME}/telegraf.ticket" --identity telegraf
  run_success "$OCKAM" node create telegraf --identity telegraf
  run_success "$OCKAM" tcp-inlet create --at /node/telegraf --from "${INFLUX_PORT}" \
    --via $relay_name --allow '(= subject.component "influxdb")'

  run_success kill_telegraf_instance
  run_success start_telegraf_instance

  # Ensure that telegraf works with using Ockam route
  run_success curl -s \
    --retry-all-errors --retry-delay 5 --retry 10 \
    --header "Authorization: Token $INFLUX_TOKEN" \
    --header "Accept: application/csv" \
    --header 'Content-type: application/vnd.flux' \
    --data "from(bucket:\"$INFLUX_BUCKET\") |> range(start:-1m)" \
    "http://localhost:$INFLUX_PORT/api/v2/query?org=$INFLUX_ORG"
}
