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

  def init() do
    settings = get_settings()

    create_node(settings.host, settings.token, settings.node_fqdn)

    attach_send_to_ui(settings.host, settings.token, settings.node_fqdn)
  end

  def get_settings() do
    token = Application.get_env(:ockam_hub, :auth_message)
    host = Application.get_env(:ockam_hub, :auth_host)

    node_fqdn = Application.get_env(:ockam_hub, :node_fqdn)

    %{
      host: host,
      token: token,
      node_fqdn: node_fqdn
    }
  end

  def attach_send_to_ui() do
    settings = get_settings()

    attach_send_to_ui(settings.host, settings.token, settings.node_fqdn)
  end

  @spec attach_send_to_ui(any, any, any) :: :ok | {:error, :already_exists}
  def attach_send_to_ui(host, token, _node_fqdn) do
    event_name = [:ockam, Ockam.Node, :handle_local_message, :start]

    handler = fn _event, _message, metadata, _options ->
      # 2. get message from metadata
      # 3. format message for JSON
      # 4. set hostname and query string from secrets
      # payload = """
      # {"message": {"version": 1,"onward_route": ["a","b","c"],"return_route": ["1","2","3"],"payload": "asdf"}}
      # """
      metadata =
        case metadata do
          %{message: %{payload: payload} = message} ->
            %{metadata | message: %{message | payload: Base.encode64(payload)}}

          other ->
            other
        end

      json_payload = Jason.encode!(metadata)
      token = URI.encode_www_form(token)

      HTTPoison.post("#{host}/messages?token=#{token}", json_payload, [
        {"Content-Type", "application/json"}
      ])
    end

    :telemetry.detach(:send_to_ui)
    :telemetry.attach(:send_to_ui, event_name, handler, nil)
  end

  def create_node() do
    settings = get_settings()

    create_node(settings.host, settings.token, settings.node_fqdn)
  end

  @spec create_node(any, any, any) :: :ok
  def create_node(host, token, node_fqdn) do
    payload =
      Jason.encode!(%{
        node: %{
          hostname: node_fqdn,
          port: 4000
        }
      })

    token = URI.encode_www_form(token)

    case HTTPoison.post("#{host}/nodes?token=#{token}", payload, [
           {"Content-Type", "application/json"}
         ]) do
      {:ok, %{status_code: 204}} ->
        :ok

      {:ok, %{status_code: 422}} ->
        Logger.info("Node already created in UI")
        :ok

      {:ok, %{status_code: code}} ->
        Logger.info("UI responds with code #{inspect(code)}")
        :ok

      {:error, %HTTPoison.Error{reason: :econnrefused}} ->
        Logger.error("connection refused trying to create a node")
        :ok

      {:error, %HTTPoison.Error{reason: reason}} ->
        Logger.error("UI request error #{inspect(reason)}")
        :ok
    end

    # we don't care if this fails,
    # we'll see it in the nginx log
    :ok
  end
end
