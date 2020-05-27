Code.require_file("support/test_cluster.exs", __DIR__)
{:ok, _} = TestCluster.start()

Application.ensure_all_started(:ockam)

ExUnit.start(capture_log: true, trace: true)
