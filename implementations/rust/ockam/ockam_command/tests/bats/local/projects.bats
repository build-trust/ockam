#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "projects - a project can be imported from a JSON file" {
  cat <<EOF >"$OCKAM_HOME/project.json"
{
  "id": "66529571-169f-44c6-8a6f-5282c1eda44c",
  "name": "awesome",
  "space_name": "together-porgy",
  "access_route": "/dnsaddr/k8s-hubdev-nginxing-9096afe9cf-7f601c51a0c807d3.elb.us-west-1.amazonaws.com/tcp/4017/service/api",
  "users": [],
  "space_id": "758623ff-5ecd-4671-8ac7-2cf269de4784",
  "identity": "I923829d0397a06fa862be5a87b7966959b8ef99ab6455b843ca9131a747b4819",
  "project_change_history": "81825837830101583285f68200815820f405e06d988fa8039cce1cd0ae607e46847c1b64bc459ca9d89dd9b21ae30681f41a654cebe91a7818eee98200815840494c9b70e8a9ad5593fceb478f722a513b4bd39fa70f4265d584253bc24617d0eb498ce532273f6d0d5326921e013696fce57c20cc6c4008f74b816810f0b009",
  "authority_access_route": "/dnsaddr/k8s-hubdev-nginxing-9096afe9cf-7f601c51a0c807d3.elb.us-west-1.amazonaws.com/tcp/4018/service/api",
  "authority_identity": "81825837830101583285f6820081582066253eb5d5ad69eac74a380293c47deb0449bb4c2d9907e51d4a481ed3dfb8c1f41a656f0afb1a783b0dfb8200815840c0f408b2164ab86b42d03ba7d3cbeffe8ba5fa13fbf32ac882ad5188414688c54076de19cb25737c120f1f8a915e10442b743012802865a9cf21dffa0197d105",
  "version": "605c4632ded93eb17edeeef31fa3860db225b3ab-2023-12-05",
  "running": true,
  "operation_id": null,
  "user_roles": [
    {
      "email": "etorreborre@gmail.com",
      "id": 28,
      "role": "Admin",
      "scope": "Space"
    }
  ]
}
EOF

  run_success "$OCKAM" project import --project-file $OCKAM_HOME/project.json
  assert_output --partial "Successfully imported project awesome"
}
