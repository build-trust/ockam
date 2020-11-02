Application.ensure_all_started(:logger)
Application.ensure_all_started(:ockam)
Application.ensure_all_started(:ockam_vault_software)

ExUnit.start(capture_log: true, trace: true)
