defmodule Ockam.Hub.KafkaStreamHandler do
  @moduledoc """
  Hub.Web API handler to create kafka stream services
  """

  def create(conn) do
    case stream_prefix(conn) do
      {:ok, stream_prefix} ->
        case Ockam.Hub.StreamSpawner.create_kafka_service(stream_prefix) do
          {:ok, service_addresses} ->
            response_body = Jason.encode!(service_addresses)
            Plug.Conn.send_resp(conn, 201, response_body)

          {:error, :disabled} ->
            Plug.Conn.send_resp(conn, 403, "Kafka integration disabled")
        end

      :error ->
        Plug.Conn.send_resp(conn, 400, "Stream prefix required")
    end
  end

  def stream_prefix(conn) do
    case conn.body_params do
      %{"stream_prefix" => stream_prefix} -> {:ok, stream_prefix}
      _other -> :error
    end
  end
end
