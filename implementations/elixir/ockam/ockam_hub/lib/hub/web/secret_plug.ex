defmodule Ockam.Hub.Web.SecretPlug do
  @moduledoc """
  Auth plug for Hub.Web HTTP API
  """
  import Plug.Conn

  def init(options), do: options

  def call(conn, _opts) do
    secret = Application.get_env(:ockam_hub, :auth_message)

    with %{"token" => token} <- conn.query_params,
         ^secret <- token do
      conn
    else
      _other ->
        conn
        |> send_resp(:unauthorized, "")
        |> halt()
    end
  end
end
