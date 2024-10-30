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

@test "remote vault - remote identity" {
  inlet_port="$(random_port)"

  ticket=$(${OCKAM} project ticket --usage-count 2)

  # start remote vault node
  run_success ${OCKAM} node create vault-node --tcp-listener-address 127.0.0.1:${inlet_port}
  run_success ${OCKAM} service start remote-proxy-vault --vault-name default

  # create a local identity, which will be used to access the remote vault, and enroll to the project
  setup_home_dir
  run_success ${OCKAM} identity create local-identity
  run_success ${OCKAM} project enroll --identity local-identity "${ticket}"
  run_success ${OCKAM} vault create \
    --identity local-identity \
    --route /ip4/127.0.0.1/tcp/${inlet_port}/secure/api/service/remote_proxy_vault \
    remote-vault
  run_success ${OCKAM} vault show remote-vault
  run_success ${OCKAM} identity create remote-identity --vault remote-vault

  # enroll remote identity
  run_success ${OCKAM} project enroll --identity remote-identity "${ticket}"

  # create a node using the remote vault and send a message to the echo service
  run_success ${OCKAM} node create using-remote-vault-node --identity remote-identity
  run_success ${OCKAM} message send --timeout 60 \
    --identity local-identity \
    --to /node/using-remote-vault-node/secure/api/service/echo \
    hello
  assert_output "hello"
  run_success ${OCKAM} message send --timeout 60 \
    --identity remote-identity \
    --to /node/using-remote-vault-node/secure/api/service/echo \
    hello
  assert_output "hello"
}

@test "remote vault - remote identity through a relay" {
  ticket=$(${OCKAM} project ticket --usage-count 2)

  # start remote vault node
  run_success ${OCKAM} node create vault-node
  run_success ${OCKAM} service start remote-proxy-vault --vault-name default
  run_success ${OCKAM} relay create vault

  # create a local identity, which will be used to access the remote vault, and enroll to the project
  setup_home_dir
  run_success ${OCKAM} identity create local-identity
  run_success ${OCKAM} project enroll --identity local-identity "${ticket}"
  run_success ${OCKAM} vault create \
    --identity local-identity \
    --route /project/default/service/forward_to_vault/secure/api/service/remote_proxy_vault \
    remote-vault

  run_success ${OCKAM} vault show remote-vault
  run_success ${OCKAM} identity create remote-identity --vault remote-vault

  # enroll remote identity
  run_success ${OCKAM} project enroll --identity remote-identity "${ticket}"

  # create a node using the remote vault and send a message to the echo service
  run_success ${OCKAM} node create using-remote-vault-node --identity remote-identity
  run_success ${OCKAM} message send --timeout 60 \
    --identity local-identity \
    --to /node/using-remote-vault-node/secure/api/service/echo \
    hello
  assert_output "hello"
  run_success ${OCKAM} message send --timeout 60 \
    --identity remote-identity \
    --to /node/using-remote-vault-node/secure/api/service/echo \
    hello
  assert_output "hello"
}
