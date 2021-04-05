defmodule Ockam.Hub.TelemetryForwarder do
  @moduledoc false

  @spec forward(any, [atom, ...], any, any) :: :ok | {:error, :already_exists}
  def forward(handler_name, event_name, node_name, process_name) do
    handler = fn ev, mes, met, opt ->
      send({process_name, node_name}, {:telemetry, {ev, mes, met, opt}})
    end

    :telemetry.attach(handler_name, event_name, handler, nil)
  end

  @spec attach_send_to_ui(any, any) :: :ok | {:error, :already_exists}
  def attach_send_to_ui(host, token) do
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

      HTTPoison.post("#{host}/messages?token=#{token}", payload, [
        {"Content-Type", "application/json"}
      ])
    end

    :telemetry.attach(:send_to_ui, event_name, handler, nil)
  end
end
