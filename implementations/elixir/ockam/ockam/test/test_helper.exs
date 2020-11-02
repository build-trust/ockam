Application.ensure_all_started(:ockam)
Application.ensure_all_started(:telemetry)

ExUnit.start(capture_log: true, trace: true)
