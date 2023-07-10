defmodule Ockam.Kafka.Interceptor.MetadataHandler do
  @moduledoc """
  Metadata response handlers for kafka interceptor.

  Support creation of inlets and outlets for metadata brokers
  """
  alias Ockam.Kafka.Interceptor.Protocol.ResponseHeader

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response, as: MetadataResponse
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response.Broker

  alias Ockam.Kafka.Interceptor.InletManager
  alias Ockam.Kafka.Interceptor.OutletManager
  alias Ockam.Kafka.Interceptor.OutletManager.Outlet

  require Logger

  def outlet_response(%ResponseHeader{}, %MetadataResponse{} = response, state) do
    Logger.info("Handle oulet metadata response")

    case create_broker_outlets(response) do
      :ok -> {:ok, state}
      {:error, reason} -> {:error, reason}
    end
  end

  def outlet_response(%ResponseHeader{api_key: api_key}, _response, state) do
    Logger.info("Ignoring response with api key #{inspect(api_key)}")
    {:ok, state}
  end

  def inlet_response(%ResponseHeader{}, %MetadataResponse{} = response, state) do
    Logger.info("Handle inlet metadata response")

    base_port =
      Map.fetch!(state, :handler_options)
      |> Keyword.get(:base_port, 9001)

    case create_broker_inlets(response, base_port) do
      {:ok, new_response} ->
        {:ok, new_response, state}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def inlet_response(%ResponseHeader{api_key: api_key}, _response, state) do
    Logger.info("Ignoring response with api key #{inspect(api_key)}")
    {:ok, state}
  end

  defp create_broker_inlets(%MetadataResponse{brokers: brokers} = response, base_port) do
    metadata_inlet_nodes =
      Enum.map(brokers, fn broker -> broker.node_id end)
      |> Enum.sort()

    existing_inlet_nodes =
      InletManager.list_inlets()
      |> Enum.map(fn {node_id, _pid} -> node_id end)
      |> Enum.sort()

    case existing_inlet_nodes == metadata_inlet_nodes do
      true ->
        :ok

      false ->
        InletManager.set_inlets(metadata_inlet_nodes)
    end

    brokers =
      Enum.map(brokers, fn %Broker{node_id: node_id} = broker ->
        ## TODO: support pointing to non-localhost host
        %{broker | host: "localhost", port: inlet_port(node_id, base_port)}
      end)

    {:ok, %{response | brokers: brokers}}
  end

  defp create_broker_outlets(%MetadataResponse{brokers: brokers}) do
    outlet_prefix = OutletManager.get_outlet_prefix()

    metadata_outlets =
      Enum.map(brokers, fn broker ->
        %Outlet{
          outlet_prefix: outlet_prefix,
          node_id: to_string(broker.node_id),
          target_host: broker.host,
          target_port: broker.port
        }
      end)
      |> Enum.sort()

    ## Quick check to not block OutletManager too much
    existing_outlets = OutletManager.get_existing_outlets(outlet_prefix)

    case metadata_outlets == existing_outlets do
      true ->
        :ok

      false ->
        OutletManager.set_outlets(metadata_outlets)
    end
  end

  ## TODO: maybe we should return the port from InletManager functions
  defp inlet_port(node_id, base_port) do
    base_port + node_id
  end
end
