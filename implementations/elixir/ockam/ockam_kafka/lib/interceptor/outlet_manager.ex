defmodule Ockam.Kafka.Interceptor.OutletManager.Outlet do
  @moduledoc """
  Data structure to use with Ockam.Kafka.Interceptor.OutletManager
  """
  defstruct [:outlet_prefix, :node_id, :target_host, :target_port]
end

defmodule Ockam.Kafka.Interceptor.OutletManager do
  @moduledoc """
  Dynamic outlet manager for kafka interceptor.

  This module serves for synchronization of outlet creation and deletion in the node.
  Multiple kafka interceptors may request outlets to be updated, last write wins.
  """
  use GenServer

  alias Ockam.Kafka.Interceptor.OutletManager.Outlet

  def start_link([outlet_prefix, ssl, ssl_options]) do
    GenServer.start_link(
      __MODULE__,
      [outlet_prefix, ssl, ssl_options],
      name: __MODULE__
    )
  end

  @impl true
  def init([outlet_prefix, ssl, ssl_options]) do
    {:ok,
     %{
       outlet_prefix: outlet_prefix,
       ssl: ssl,
       ssl_options: ssl_options
     }}
  end

  def get_outlet_prefix(server \\ __MODULE__, timeout \\ 5000) do
    GenServer.call(server, :get_outlet_prefix, timeout)
  end

  def get_outlets(server \\ __MODULE__, timeout \\ 5000) do
    GenServer.call(server, :get_outlets, timeout)
  end

  def set_outlets(server \\ __MODULE__, outlets, timeout \\ 5000) when is_list(outlets) do
    GenServer.call(server, {:set_outlets, outlets}, timeout)
  end

  @impl true
  def handle_call(:get_outlet_prefix, _from, %{outlet_prefix: outlet_prefix} = state) do
    {:reply, outlet_prefix, state}
  end

  def handle_call(:get_outlets, _from, %{outlet_prefix: outlet_prefix} = state) do
    {:reply, get_existing_outlets(outlet_prefix), state}
  end

  ## TODO: maybe we want to terminate existing connections when outlets are reshuffled??
  def handle_call({:set_outlets, outlets}, _from, state) do
    %{outlet_prefix: outlet_prefix} = state
    existing_outlets = get_existing_outlets(outlet_prefix)
    outlets = Enum.sort(outlets)

    case outlets == existing_outlets do
      true ->
        {:reply, :ok, state}

      false ->
        to_stop = existing_outlets -- outlets
        to_start = outlets -- existing_outlets

        Enum.each(to_stop, fn outlet ->
          stop_outlet(outlet)
        end)

        Enum.each(to_start, fn outlet ->
          start_outlet(outlet, state)
        end)

        {:reply, :ok, state}
    end
  end

  def get_existing_outlets(outlet_prefix) do
    Ockam.Node.list_addresses()
    |> Enum.filter(fn address -> String.starts_with?(address, outlet_prefix) end)
    |> Enum.flat_map(fn address ->
      ## TODO: explicit API to fetch worker options from outlet
      case Ockam.Node.whereis(address) do
        nil ->
          []

        pid when is_pid(pid) ->
          [Map.take(:sys.get_state(pid), [:address, :worker_options])]
      end
    end)
    |> Enum.map(fn %{address: address, worker_options: options} ->
      target_host = Keyword.fetch!(options, :target_host)
      target_port = Keyword.fetch!(options, :target_port)

      %Outlet{
        outlet_prefix: outlet_prefix,
        node_id: String.replace_prefix(address, outlet_prefix, ""),
        target_host: target_host,
        target_port: target_port
      }
    end)
    |> Enum.sort()
  end

  defp outlet_address(node_id, outlet_prefix) do
    outlet_prefix <> to_string(node_id)
  end

  defp stop_outlet(%Outlet{node_id: node_id, outlet_prefix: outlet_prefix}) do
    Ockam.Node.stop(outlet_address(node_id, outlet_prefix))
  end

  defp start_outlet(
         %Outlet{
           node_id: node_id,
           outlet_prefix: outlet_prefix,
           target_host: target_host,
           target_port: target_port
         },
         %{ssl: ssl, ssl_options: ssl_options}
       ) do
    address = outlet_address(node_id, outlet_prefix)
    ## We crash on failures because error handling would be too complex
    ## TODO: see if we can propagate the error
    {:ok, _pid, _extra} =
      Ockam.Session.Spawner.start_link(
        address: address,
        worker_mod: Ockam.Transport.Portal.OutletWorker,
        worker_options: [
          target_host: target_host,
          target_port: target_port,
          ssl: ssl,
          ssl_options: ssl_options
        ]
      )
  end
end
