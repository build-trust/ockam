defmodule Ockam.Hub.Web.Router do
  @moduledoc """
  Router for Hub.Web HTTP API
  """
  use Plug.Router

  alias Ockam.Hub.KafkaStreamHandler

  def child_spec(options) do
    Plug.Cowboy.child_spec(
      scheme: :http,
      plug: __MODULE__,
      options: [port: Keyword.fetch!(options, :port)]
    )
  end

  plug(:match)
  plug(Plug.Parsers, parsers: [:json], json_decoder: Jason)
  plug(Ockam.Hub.Web.SecretPlug)
  plug(:dispatch)

  post "/streams" do
    KafkaStreamHandler.create(conn)
  end
end
