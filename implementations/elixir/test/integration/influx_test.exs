defmodule Ockam.Integration.Influx.Test do
  use ExUnit.Case, async: false
  require Logger

  alias Ockam.Vault
  alias Ockam.Transport.Socket
  alias Ockam.Test.Support.Influx.Client

  setup context do
    {:ok, vault} = Vault.new()

    _ = start_supervised!({Ockam.Registry, []})

    if transport = context[:transport] do
      # Make sure we have a router started
      name = Map.fetch!(context, :transport_name)
      meta = [name: name]
      config = Map.get(context, :transport_config, [])
      pid = start_supervised!({transport, [meta, config]})

      # Start the Influx service
      influx_meta = [
        name: {:via, Registry, {Ockam.Registry, "influx_example", Ockam.Services.Influx}}
      ]

      influx_opts = [database: "test", http: [host: "127.0.0.1", port: 8086]]
      _ = start_supervised!({Ockam.Services.Influx, [influx_meta, influx_opts]})

      {:ok, [vault: vault, pid: pid, service: :influx_example, config: config]}
    else
      {:ok, [vault: vault]}
    end
  end

  @tag skip: true
  @tag transport: Ockam.Transport.TCP
  @tag transport_name: :tcp_4005
  @tag transport_config: [listen_address: "0.0.0.0", listen_port: 4005]
  test "influx test", %{vault: vault, config: config} = context do
    client_opts = [{:service, context[:service]} | config]
    assert {:ok, client} = Client.new(vault, client_opts)
    assert {:ok, client} = Client.connect(client)

    # We're connected, so write a few data points
    assert {:ok, client} = Client.write(client, "temps", [loc: "home"], temp: 72)
    assert {:ok, client} = Client.write(client, "temps", [loc: "home"], temp: 82)
    assert {:ok, client} = Client.write(client, "temps", [loc: "home"], temp: 76)

    Process.sleep(1_000)

    # Query the data we just generated
    assert {:ok, client, results} =
             Client.query(client, "SELECT \"temp\" FROM \"temps\" WHERE \"loc\"='home'")

    assert %{:results => [%{:series => [%{:values => values}]}]} = results

    refute nil == Enum.find(values, fn [_timestamp, temp] -> temp in [72, 76, 82] end)

    assert {:ok, _} = client |> Client.transport() |> Socket.close()
  end
end
