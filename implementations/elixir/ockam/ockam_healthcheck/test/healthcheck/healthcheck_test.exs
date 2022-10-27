defmodule Ockam.Tests do
  use ExUnit.Case, async: true

  require Logger

  setup_all do
    {:ok, transport} = Ockam.Transport.TCP.start(listen: [port: 4000])

    {:ok, _api} =
      Ockam.Identity.SecureChannel.create_listener(
        identity: :dynamic,
        address: "api",
        trust_policies: []
      )

    {:ok, _pid, _ping} = Ockam.Services.Echo.start_link(address: "healthcheck")

    on_exit(fn ->
      Ockam.Node.stop("api")
      Ockam.Node.stop("healthcheck")
      GenServer.stop(transport)
    end)

    :ok
  end

  test "healthcheck OK" do
    node_host = "localhost"
    node_port = 4000
    api_worker = "api"
    ping_worker = "healthcheck"
    assert :ok = Ockam.Healthcheck.check_node(node_host, node_port, api_worker, ping_worker, 1000)
  end

  test "healthcheck ping error" do
    node_host = "localhost"
    node_port = 4000
    api_worker = "api"
    ping_worker = "not_healthcheck"

    assert {:error, :timeout} =
             Ockam.Healthcheck.check_node(node_host, node_port, api_worker, ping_worker, 1000)
  end

  test "healthcheck channel error" do
    node_host = "localhost"
    node_port = 4000
    api_worker = "not_api"
    ping_worker = "healthcheck"

    assert {:error, :key_exchange_timeout} =
             Ockam.Healthcheck.check_node(node_host, node_port, api_worker, ping_worker, 1000)
  end

  test "healthcheck TCP error" do
    node_host = "localhost"
    node_port = 1234
    api_worker = "api"
    ping_worker = "healthcheck"

    assert {:error, {:tcp_connection_error, :econnrefused}} =
             Ockam.Healthcheck.check_node(node_host, node_port, api_worker, ping_worker, 1000)
  end
end
