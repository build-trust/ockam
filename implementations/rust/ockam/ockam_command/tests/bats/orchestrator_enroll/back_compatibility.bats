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

@test "backcompat - generate ticket, enroll with old version" {
  latest_version=$($OCKAM --version | grep -o 'ockam [0-9]*\.[0-9]*\.[0-9]*' | sed 's/ockam //')
  latest_minor_version=$(echo $latest_version | cut -d. -f2)
  versions_to_check=$(seq -f "0.%g.0" $((latest_minor_version - 5)) $((latest_minor_version - 1)))
  for version in $versions_to_check; do
    echo "Testing enrollment ticket with ockam version $version" >&3
    container_name=$(random_str)
    port=$(random_port)
    node_name=$(random_str)
    relay_name=$(random_str)
    docker run -d --name $container_name -p $port:$port \
      ghcr.io/build-trust/ockam:$version node create --foreground \
      --enrollment-ticket "$($OCKAM project ticket --relay $relay_name)" \
      --configuration "{name: $node_name, relay: $relay_name}" \
      -vvv
    sleep 10
    run $OCKAM message send hi --to /project/default/service/forward_to_${relay_name}/secure/api/service/echo
    response=$output
    docker kill $container_name
    docker rm $container_name
    assert_equal "$response" "hi"
  done
}
