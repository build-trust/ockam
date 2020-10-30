Application.ensure_all_started(:ockam)
Application.ensure_all_started(:ockam_hub)

ExUnit.start(capture_log: true, trace: true)
