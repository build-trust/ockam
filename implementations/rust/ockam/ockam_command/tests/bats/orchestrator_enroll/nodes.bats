#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "nodes - create with config, admin enrolling twice with the project doesn't return error" {
  $OCKAM project ticket --usage-count 10 >"$OCKAM_HOME/enrollment.ticket"

  cat <<EOF >"$OCKAM_HOME/config.yaml"
name: n1
EOF

  ## The default identity is already enrolled, so the enrollment step should be skipped
  run_success "$OCKAM" node create "$OCKAM_HOME/config.yaml" \
    --enrollment-ticket "$OCKAM_HOME/enrollment.ticket"
  run_success "$OCKAM" message send hello --timeout 5 --to "/node/n1/secure/api/service/echo"
}

@test "nodes - create with config, non-admin enrolling twice with the project doesn't return error" {
  ADMIN_HOME_DIR="$OCKAM_HOME"
  ticket_path="$ADMIN_HOME_DIR/enrollment.ticket"
  export RELAY_NAME=$(random_str)
  $OCKAM project ticket --usage-count 10 --relay $RELAY_NAME >"$ticket_path"

  # User: try to enroll the same identity twice
  setup_home_dir
  export CLIENT_PORT=$(random_port)

  ## First time it works
  run_success "$OCKAM" node create "$BATS_TEST_DIRNAME/fixtures/node-create.basic.config.yaml" \
    --enrollment-ticket "$ticket_path" \
    --variable SERVICE_PORT="$PYTHON_SERVER_PORT"
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$CLIENT_PORT"

  ## Second time it will skip the enrollment step and the node will be set up as expected
  run_success "$OCKAM" node delete --all -y
  run_success "$OCKAM" node create "$BATS_TEST_DIRNAME/fixtures/node-create.basic.config.yaml" \
    --enrollment-ticket "$ticket_path" \
    --variable SERVICE_PORT="$PYTHON_SERVER_PORT"
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$CLIENT_PORT"
}

@test "nodes - create with config in foreground" {
  # Admin: create enrollment ticket
  ADMIN_HOME_DIR="$OCKAM_HOME"
  ticket_path="$ADMIN_HOME_DIR/enrollment.ticket"
  export RELAY_NAME=$(random_str)
  $OCKAM project ticket --usage-count 10 --relay $RELAY_NAME >"$ticket_path"

  # User: create a node in the foreground with a portal and using an enrollment ticket
  setup_home_dir
  export CLIENT_PORT=$(random_port)

  ## Create node and try to reach it
  run_success "$OCKAM" node create "$BATS_TEST_DIRNAME/fixtures/node-create.basic.config.yaml" \
    --foreground \
    --enrollment-ticket "$ticket_path" \
    --variable SERVICE_PORT="$PYTHON_SERVER_PORT" &
  sleep 10
  run_success "$OCKAM" message send hello --timeout 2 --to "/node/n1/secure/api/service/echo"
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$CLIENT_PORT"
}

@test "nodes - create with config, single machine, unnamed portal" {
  export RELAY_NAME=$(random_str)
  export NODE_PORT=$(random_port)
  export CLIENT_PORT=$(random_port)

  run_success "$OCKAM" node create "$BATS_TEST_DIRNAME/fixtures/node-create.1.unnamed-portal.config.yaml" \
    --variable SERVICE_PORT="$PYTHON_SERVER_PORT"

  # node created with expected name
  run_success "$OCKAM" message send --timeout 5 hello --to "/node/n1/secure/api/service/echo"
  # tcp-listener-address set to expected port
  run_success "$OCKAM" message send --timeout 5 hello --to "/dnsaddr/127.0.0.1/tcp/$NODE_PORT/secure/api/service/echo"
  # portal is working: inlet -> relay -> outlet -> python server
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$CLIENT_PORT"
}

@test "nodes - create with config, single machine, named portal" {
  export RELAY_NAME=$(random_str)
  export CLIENT_PORT=$(random_port)
  export NODE_PORT=$(random_port)

  run_success "$OCKAM" node create "$BATS_TEST_DIRNAME/fixtures/node-create.2.named-portal.config.yaml" \
    --variable SERVICE_PORT="$PYTHON_SERVER_PORT"

  # node created with expected name
  run_success "$OCKAM" message send --timeout 5 hello --to "/node/n1/secure/api/service/echo"
  # tcp-listener-address set to expected port
  run_success "$OCKAM" message send --timeout 5 hello --to "/dnsaddr/127.0.0.1/tcp/$NODE_PORT/secure/api/service/echo"
  # portal is working: inlet -> relay -> outlet -> python server
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$CLIENT_PORT"
}

@test "nodes - create with config, multiple machines" {
  skip "Temporary disabled due to some upcoming changes in the dev env of the Orchestrator"
  ADMIN_HOME_DIR="$OCKAM_HOME"
  export SAAS_RELAY_NAME=$(random_str)
  # Admin: create enrollment ticket for SaaS
  $OCKAM project ticket \
    --attribute "ockam-role=enroller" --attribute "to-saas=outlet" --attribute "from-saas=inlet" \
    --relay "to-$SAAS_RELAY_NAME" --usage-count 10 >"$ADMIN_HOME_DIR/saas.ticket"

  # SaaS: create portal + enrollment ticket for Customer
  setup_home_dir
  SAAS_HOME_DIR="$OCKAM_HOME"

  ## The portal ports are constants in the SaaS machine, so we can export them
  export SAAS_INLET_PORT=$(random_port)
  export SAAS_OUTLET_PORT=$PYTHON_SERVER_PORT

  ## The customer details are variables that will change everytime the SaaS wants to add a new customer
  customer_name=$(random_str)
  customer_service="myapp"

  run_success "$OCKAM" node create "$BATS_TEST_DIRNAME/fixtures/node-create.3.saas.config.yaml" \
    --enrollment-ticket "$ADMIN_HOME_DIR/saas.ticket" \
    --variable CUSTOMER="$customer_name" \
    --variable CUSTOMER_SERVICE_NAME="$customer_service"

  $OCKAM project ticket \
    --attribute "to-saas=inlet" --attribute "from-saas=outlet" \
    --relay "to-$customer_name" --usage-count 10 >"$SAAS_HOME_DIR/$customer_name.ticket"

  # Customer: create portal
  setup_home_dir

  ## Similarly, we export the constant variables for the Customer
  export CUSTOMER="$customer_name"
  export CUSTOMER_INLET_PORT=$(random_port)
  export CUSTOMER_OUTLET_PORT=$(random_port)
  export CUSTOMER_SERVICE_NAME="$customer_service"

  run_success "$OCKAM" node create "$BATS_TEST_DIRNAME/fixtures/node-create.3.customer.config.yaml" \
    --enrollment-ticket "$SAAS_HOME_DIR/$customer_name.ticket"

  # Test: SaaS service can be reached from Customer's inlet
  $OCKAM message send hi --to "/project/default/service/forward_to_to-$SAAS_RELAY_NAME/secure/api/service/echo"
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$CUSTOMER_INLET_PORT"

  # Test: Customer node can be reached from SaaS's side
  export OCKAM_HOME="$SAAS_HOME_DIR"
  $OCKAM message send hi --to "/project/default/service/forward_to_to-$CUSTOMER/secure/api/service/echo"
}

@test "nodes - create with config, download config and enrollment-ticket from URL" {
  random_file_name=$(random_str)
  ticket_relative_path=".tmp/$random_file_name.ticket"
  $OCKAM project ticket --usage-count 10 >"$OCKAM_HOME_BASE/$ticket_relative_path"

  # Create a config file in the python server's root directory
  config_relative_path=".tmp/$random_file_name.config.yaml"
  cat <<EOF >"$OCKAM_HOME_BASE/$config_relative_path"
name: n1
EOF

  # Using a proper url (with scheme)
  run_success "$OCKAM" node create "http://127.0.0.1:$PYTHON_SERVER_PORT/$config_relative_path" \
    --enrollment-ticket "http://127.0.0.1:$PYTHON_SERVER_PORT/$ticket_relative_path"
  run_success "$OCKAM" message send --timeout 5 hello --to "/node/n1/secure/api/service/echo"

  # Without a scheme
  run_success "$OCKAM" node delete --all -y
  run_success "$OCKAM" node create "127.0.0.1:$PYTHON_SERVER_PORT/$config_relative_path" \
    --enrollment-ticket "127.0.0.1:$PYTHON_SERVER_PORT/$ticket_relative_path"
  run_success "$OCKAM" message send --timeout 5 hello --to "/node/n1/secure/api/service/echo"
}

@test "nodes - create with config, using the specified identity" {
  export RELAY_NAME=$(random_str)
  $OCKAM project ticket --relay "$RELAY_NAME" >"$OCKAM_HOME/enrollment.ticket"

  cat <<EOF >"$OCKAM_HOME/config.yaml"
name: n1
identity: i1
relay: $RELAY_NAME
EOF

  # The identity will be created and enrolled
  run_success "$OCKAM" node create "$OCKAM_HOME/config.yaml" \
    --enrollment-ticket "$OCKAM_HOME/enrollment.ticket"

  # Use the identity to send a message
  $OCKAM message send hi --identity i1 --to "/project/default/service/forward_to_$RELAY_NAME/secure/api/service/echo"
}

@test "nodes - create with config, using the specified enrollment ticket" {
  $OCKAM project ticket >"$OCKAM_HOME/enrollment.ticket"

  # The identity will be enrolled
  run_success "$OCKAM" node create n1 --identity i1 --enrollment-ticket "$OCKAM_HOME/enrollment.ticket"

  # Check that the identity can reach the project
  run_success $OCKAM message send hi --identity i1 --to "/project/default/service/echo"
}

@test "nodes - create with config, using the specified enrollment ticket, overriding config" {
  $OCKAM project ticket >"$OCKAM_HOME/enrollment.ticket"

  cat <<EOF >"$OCKAM_HOME/config.yaml"
ticket: other.ticket
name: n2
identity: i2
EOF

  # The values from the config file will be overridden by the command line arguments
  run_success "$OCKAM" node create n1 --identity i1 --enrollment-ticket "$OCKAM_HOME/enrollment.ticket"
  run_failure "$OCKAM" node show n2
  run_failure "$OCKAM" identity show i2

  # Check that the identity can reach the project
  run_success $OCKAM message send hi --identity i1 --to "/project/default/service/echo"
}

@test "nodes - create with config, using the specified enrollment ticket as an env var" {
  $OCKAM project ticket >"$OCKAM_HOME/enrollment.ticket"
  export ENROLLMENT_TICKET=$(cat "$OCKAM_HOME/enrollment.ticket")

  cat <<EOF >"$OCKAM_HOME/config.yaml"
ticket: ${ENROLLMENT_TICKET}
name: n1
EOF

  run_success "$OCKAM" node create "$OCKAM_HOME/config.yaml"
  run_success "$OCKAM" node show n1

  # Check that the identity can reach the project
  run_success $OCKAM message send hi --to "/project/default/service/echo"
}

@test "nodes - create with config, using the specified enrollment ticket as an env var, in foreground" {
  $OCKAM project ticket >"$OCKAM_HOME/enrollment.ticket"
  export ENROLLMENT_TICKET=$(cat "$OCKAM_HOME/enrollment.ticket")

  cat <<EOF >"$OCKAM_HOME/config.yaml"
ticket: ${ENROLLMENT_TICKET}
name: n1
EOF

  run_success "$OCKAM" node create "$OCKAM_HOME/config.yaml" -f &
  sleep 10
  run_success "$OCKAM" node show n1

  # Check that the identity can reach the project
  run_success $OCKAM message send hi --to "/project/default/service/echo"
}
