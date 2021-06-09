defmodule Ockam.Hub.KafkaStreamHandler do
  @moduledoc """
  Hub.Web API handler to create kafka stream services
  """

  alias Ockam.Hub.Service.Provider, as: ServiceProvider

  require Logger

  def create(conn) do
    case stream_prefix(conn) do
      {:ok, stream_prefix} ->
        case start_services(stream_prefix) do
          {:ok, services} ->
            response_body = Jason.encode!(services)

            Plug.Conn.send_resp(conn, 201, response_body)

          {:error, :disabled} ->
            Plug.Conn.send_resp(conn, 403, "Kafka integration disabled")

          other ->
            Logger.error("Unable to create kafka services: #{inspect(other)}")
            Plug.Conn.send_resp(conn, 500, "Internal error")
        end

      :error ->
        Plug.Conn.send_resp(conn, 400, "Stream prefix required")
    end
  end

  def start_services(stream_prefix) do
    case services_enabled() do
      true ->
        with {:ok, stream_service} <-
               ServiceProvider.start_configured_service(:stream_kafka,
                 address_prefix: stream_prefix,
                 stream_prefix: stream_prefix
               ),
             {:ok, index_service} <-
               ServiceProvider.start_configured_service(:stream_kafka_index,
                 address_prefix: stream_prefix,
                 stream_prefix: stream_prefix
               ) do
          {:ok,
           %{
             stream_service: stream_service,
             index_service: index_service
           }}
        end

      false ->
        {:error, :disabled}
    end
  end

  def services_enabled() do
    services = ServiceProvider.get_services()
    Keyword.has_key?(services, :stream_kafka) and Keyword.has_key?(services, :stream_kafka_index)
  end

  def stream_prefix(conn) do
    case conn.body_params do
      %{"stream_prefix" => stream_prefix} -> {:ok, stream_prefix}
      _other -> :error
    end
  end
end
