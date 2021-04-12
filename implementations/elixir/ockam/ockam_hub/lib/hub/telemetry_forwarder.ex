defmodule Ockam.Hub.TelemetryForwarder do
  @moduledoc false

  require Logger

  @spec forward(any, [atom, ...], any, any) :: :ok | {:error, :already_exists}
  def forward(handler_name, event_name, node_name, process_name) do
    handler = fn ev, mes, met, opt ->
      send({process_name, node_name}, {:telemetry, {ev, mes, met, opt}})
    end

    :telemetry.attach(handler_name, event_name, handler, nil)
  end

  @spec attach_send_to_ui(any, any, any) :: :ok | {:error, :already_exists}
  def attach_send_to_ui(host, token, public_ip) do
    event_name = [:ockam, Ockam.Node, :handle_routed_message, :start]

    handler = fn _event, _message, metadata, _options ->
      # 2. get message from metadata
      # 3. format message for JSON
      # 4. set hostname and query string from secrets
      # payload = """
      # {"message": {"version": 1,"onward_route": ["a","b","c"],"return_route": ["1","2","3"],"payload": "asdf"}}
      # """
      payload = Jason.encode!(metadata)
      token = URI.encode_www_form(token)

      HTTPoison.post("#{host}/messages?token=#{token}&public_ip=#{public_ip}", payload, [
        {"Content-Type", "application/json"}
      ])
    end

    :telemetry.attach(:send_to_ui, event_name, handler, nil)
  end

  @spec create_node(any, any, any) :: :ok
  def create_node(host, token, public_ip) do
    payload =
      Jason.encode!(%{
        node: %{
          ip: public_ip,
          hostname: "auto-added",
          port: 4000
        }
      })

    case HTTPoison.post("#{host}/nodes?token=#{token}&public_ip=#{public_ip}", payload, [
           {"Content-Type", "application/json"}
         ]) do
      {:ok, %{status_code: 204}} ->
        :ok

      {:ok, %{status_code: 422}} ->
        Logger.info("Node already created in UI")
        :ok

      {:error, %HTTPoison.Error{reason: :econnrefused}} ->
        Logger.error("connection refused trying to create a node")
        :ok
    end

    # we don't care if this fails,
    # we'll see it in the nginx log
    :ok
  end
end
