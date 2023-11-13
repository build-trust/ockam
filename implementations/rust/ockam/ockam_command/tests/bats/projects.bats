#!/bin/bash

# ===== SETUP

setup() {
  load load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "projects - a project can be imported" {
  run_success bash -c "$OCKAM project import \
      --project-name awesome \
      --project-id 1 \
      --project-identifier Ie92f183eb4c324804ef4d62962dea94cf095a265a1b2c3d4e5f6a6b5c4d3e2f1 \
      --project-access-route /dnsaddr/127.0.0.1/tcp/4000/service/api \
      --authority-identity 81825837830101583285f68200815820f02eb8b3f7b97e73f4866cc76953e0fe8aa8765b69bac1bc630b3756c587aa9bf41a6565bf201a7831c2208200815840f2474f917cac6a315a780034ec54786be9368ea0e50b713eb4847571efca8f98ece6f470ef7d18deefc134752db175e7f40e154b1a7c002d9b29db0c65892a08 \
      --authority-access-route /dnsaddr/127.0.0.1/tcp/5000/service/api"

  assert_output --partial "Successfully imported project awesome"
}
