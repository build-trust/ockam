Application.ensure_all_started(:ockam)
Application.ensure_all_started(:ockam_transport_udp)

ExUnit.start(capture_log: true, trace: true)
