Application.ensure_all_started(:ockam_node_web)

ExUnit.start(capture_log: true, trace: true)
