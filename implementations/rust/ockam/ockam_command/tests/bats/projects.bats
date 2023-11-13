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
      --project-identifier I6c20e814b56579306f55c64e8747e6c1b4a53d9a \
      --project-access-route /dnsaddr/127.0.0.1/tcp/4000/service/api \
      --authority-identity 81a201583ba20101025835a4028201815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef403f4041a64dd404a051a77a9434a0282018158407754214545cda6e7ff49136f67c9c7973ec309ca4087360a9f844aac961f8afe3f579a72c0c9530f3ff210f02b7c5f56e96ce12ee256b01d7628519800723805 \
      --authority-access-route /dnsaddr/127.0.0.1/tcp/5000/service/api"

  assert_output --partial "Successfully imported project awesome"
}
