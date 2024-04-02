#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  get_project_data
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "projects - enable and disable addons" {
  skip # TODO: wait until cloud has the influxdb and confluent addons enabled

  run_success "$OCKAM" project addon list --project default
  assert_output --partial --regex "Id: okta\n +Enabled: false"
  assert_output --partial --regex "Id: confluent\n +Enabled: false"

  run_success "$OCKAM" project addon enable okta --project default --tenant tenant --client-id client_id --cert cert
  run_success "$OCKAM" project addon enable confluent --project default --bootstrap-server bootstrap-server.confluent:9092 --api-key ApIkEy --api-secret ApIsEcrEt

  run_success "$OCKAM" project addon list --project default
  assert_output --partial --regex "Id: okta\n +Enabled: true"
  assert_output --partial --regex "Id: confluent\n +Enabled: true"

  run_success "$OCKAM" project addon disable --addon okta --project default
  run_success "$OCKAM" project addon disable --addon --project default
  run_success "$OCKAM" project addon disable --addon confluent --project default

  run_success "$OCKAM" project addon list --project default
  assert_output --partial --regex "Id: okta\n +Enabled: false"
  assert_output --partial --regex "Id: confluent\n +Enabled: false"
}

@test "influxdb lease manager" {
  # TODO add more tests
  #      responsible, and that a member enrolled on a different ockam install can access it.
  skip_if_influxdb_test_not_enabled

  run_success "$OCKAM" project addon configure influxdb --org-id "${INFLUXDB_ORG_ID}" --token "${INFLUXDB_TOKEN}" --endpoint-url "${INFLUXDB_ENDPOINT}" --max-ttl 60 --permissions "${INFLUXDB_PERMISSIONS}"

  sleep 30 #FIXME  workaround, project not yet ready after configuring addon

  ADMIN_HOME=$OCKAM_HOME

  setup_home_dir
  USER_HOME=$OCKAM_HOME
  run_success "$OCKAM" project import --project-file $PROJECT_PATH

  run_success "$OCKAM" identity create m1
  run_success "$OCKAM" identity create m2
  run_success "$OCKAM" identity create m3

  m1_identifier=$($OCKAM identity show m1)
  m2_identifier=$($OCKAM identity show m2)

  export OCKAM_HOME=$ADMIN_HOME
  run_success "$OCKAM" project-member add $m1_identifier --attribute service=sensor
  run_success "$OCKAM" project-member add $m2_identifier --attribute service=web

  export OCKAM_HOME=$USER_HOME

  # m1 and m2 identity was added by enroller
  run_success "$OCKAM" project enroll --identity m1
  assert_output --partial $green_identifier

  run_success "$OCKAM" project enroll --identity m2
  assert_output --partial $green_identifier

  # m1 and m2 can use the lease manager
  run_success "$OCKAM" lease --identity m1 create
  run_success "$OCKAM" lease --identity m2 create

  # m3 can't
  run_success "$OCKAM" lease --identity m3 create
  assert_failure

  export OCKAM_HOME=$ADMIN_HOME
  run_success "$OCKAM" project addon configure influxdb --org-id "${INFLUXDB_ORG_ID}" --token "${INFLUXDB_TOKEN}" --endpoint-url "${INFLUXDB_ENDPOINT}" --max-ttl 60 --permissions "${INFLUXDB_PERMISSIONS}" --user-access-role '(= subject.service "sensor")'

  sleep 30 #FIXME  workaround, project not yet ready after configuring addon

  export OCKAM_HOME=$USER_HOME

  # m1 can use the lease manager (it has a service=sensor attribute attested by authority)
  run_success "$OCKAM" lease --identity m1 create

  # m2 can't use the  lease manager now (it doesn't have a service=sensor attribute attested by authority)
  run_failure "$OCKAM" lease --identity m2 create
}
