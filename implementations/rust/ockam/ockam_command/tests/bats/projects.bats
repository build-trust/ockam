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

@test "projects - a project can be imported from a JSON file" {
  cat <<EOF >>"$OCKAM_HOME/project.json"
{
  "id": "66529571-169f-44c6-8a6f-5282c1eda44c",
  "name": "awesome",
  "space_name": "together-porgy",
  "access_route": "/dnsaddr/k8s-hubdev-nginxing-9096afe9cf-7f601c51a0c807d3.elb.us-west-1.amazonaws.com/tcp/4017/service/api",
  "users": [],
  "space_id": "758623ff-5ecd-4671-8ac7-2cf269de4784",
  "identity": "I6443a2339360a81ce37b3ea14185d118e128ad3bd7e420ac1491937d10e026e6",
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
