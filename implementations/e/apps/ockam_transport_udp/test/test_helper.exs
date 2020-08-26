Application.ensure_all_started(:ockam)

ExUnit.start(capture_log: true, trace: true)
