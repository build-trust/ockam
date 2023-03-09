defmodule Ockam.Kafka.Interceptor.InletManager do
  @moduledoc """
  Dynamic inlet manager for kafka interceptor.

  Inlets are GenServers and cannot be managed by ockam registry.
  This module can dynamically create and stop inlets using base port and port offset.
  """
  use GenServer

  require Logger

  def start_link([base_port, allowed_ports, base_route, outlet_prefix], name \\ __MODULE__) do
    GenServer.start_link(
      __MODULE__,
      [base_port, allowed_ports, base_route, outlet_prefix],
      ## TODO: make this optional
      name: name
    )
  end

  @impl true
  def init([base_port, allowed_ports, base_route, outlet_prefix]) do
    Process.flag(:trap_exit, true)

    {:ok,
     %{
       base_port: base_port,
       allowed_ports: allowed_ports,
       base_route: base_route,
       outlet_prefix: outlet_prefix,
       inlets: %{}
     }}
  end

  def list_inlets(server \\ __MODULE__, timeout \\ 5000) do
    GenServer.call(server, :list_inlets, timeout)
  end

  def set_inlets(server \\ __MODULE__, port_offsets, timeout \\ 5000)
      when is_list(port_offsets) do
    GenServer.call(server, {:set_inlets, port_offsets}, timeout)
  end

  @impl true
  def handle_call(:list_inlets, _from, %{inlets: inlets} = state) do
    {:reply, inlets, state}
  end

  def handle_call(
        {:set_inlets, requested_port_offsets},
        _from,
        %{allowed_ports: allowed_ports, inlets: inlets} = state
      ) do
    case Enum.any?(requested_port_offsets, fn port_offset ->
           port_offset >= allowed_ports or port_offset < 0
         end) do
      true ->
        {:reply, {:error, :port_out_of_range}, state}

      false ->
        existing_port_offsets =
          inlets
          |> Enum.map(fn {port_offset, _pid} -> port_offset end)
          |> Enum.sort()

        to_stop = existing_port_offsets -- requested_port_offsets

        state =
          Enum.reduce(to_stop, state, fn port_offset, state ->
            {:ok, state} = stop_inlet(port_offset, state)
            state
          end)

        to_start = requested_port_offsets -- existing_port_offsets

        state =
          Enum.reduce(to_start, state, fn port_offset, state ->
            {:ok, state} = start_inlet(port_offset, state)
            state
          end)

        {:reply, :ok, state}
    end
  end

  @impl true
  def handle_info({:EXIT, from, :normal}, state) do
    Logger.debug("Received exit :normal signal from #{inspect(from)}")
    {:noreply, state}
  end

  def handle_info({:EXIT, _from, reason}, state) do
    {:stop, reason, state}
  end

  @impl true
  def terminate(reason, state) do
    Logger.info("Stopping inlet manager: #{inspect(reason)}")
    cleanup_inlets(state)
    :ok
  end

  defp cleanup_inlets(%{inlets: inlets} = state) do
    Enum.reduce(inlets, state, fn {port_offset, _pid}, state ->
      {:ok, state} = stop_inlet(port_offset, state)
      state
    end)
  end

  defp stop_inlet(port_offset, %{inlets: inlets} = state) do
    case Map.fetch(inlets, port_offset) do
      {:ok, pid} ->
        ## TODO: manage inlets in a supervisor
        try do
          GenServer.stop(pid)
        catch
          :exit, {:noproc, _} ->
            :ok
        end

        {:ok, %{state | inlets: Map.delete(inlets, port_offset)}}

      :error ->
        {:ok, state}
    end
  end

  defp start_inlet(port_offset, %{inlets: inlets} = state) do
    port = inlet_port(port_offset, state)
    peer_route = peer_route(port_offset, state)

    case Ockam.Transport.Portal.InletListener.start_link(port: port, peer_route: peer_route) do
      {:ok, pid} ->
        {:ok, %{state | inlets: Map.put(inlets, port_offset, pid)}}

      {:ok, pid, _extra} ->
        {:ok, %{state | inlets: Map.put(inlets, port_offset, pid)}}

      ## TODO: should we fail on :already_started?
      {:error, reason} ->
        {:error, reason}
    end
  end

  defp inlet_port(port_offset, state) do
    %{base_port: base_port} = state
    port_offset + base_port
  end

  defp peer_route(port_offset, state) do
    %{base_route: base_route, outlet_prefix: outlet_prefix} = state
    outlet_address = outlet_prefix <> to_string(port_offset)
    base_route ++ [outlet_address]
  end
end
