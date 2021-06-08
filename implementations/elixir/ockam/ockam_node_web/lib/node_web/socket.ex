defmodule Ockam.Node.Web.Socket do
  @moduledoc false

  @behaviour :cowboy_websocket

  def init(req, state) do
    {:cowboy_websocket, req, state}
  end

  def websocket_init(_state) do
    state = %{}
    {:ok, state}
  end

  def websocket_handle({:text, message}, state) do
    json = Jason.decode!(message)
    websocket_handle({:json, json}, state)
  end

  def websocket_handle({:json, _}, state) do
    {:reply, {:text, "hello world"}, state}
  end

  def websocket_info(_info, state) do
    {:ok, state}
  end

  def terminate(_reason, _req, _state) do
    :ok
  end
end
