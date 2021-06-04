defmodule Test.Hub.Web.WebTest do
  use ExUnit.Case
  use Plug.Test

  alias Ockam.Hub.Web.Router

  @options Router.init([])

  @http_status_bad_request 400
  @http_status_unauthorized 401
  @http_status_forbidden 403

  test "create stream without authorization" do
    conn =
      :post
      |> conn("/streams", %{"stream_prefix" => "prefix"})
      |> Router.call(@options)

    assert conn.status == @http_status_unauthorized
  end

  test "create stream with prefix" do
    conn =
      :post
      |> conn(create_url("/streams"), %{"stream_prefix" => "prefix"})
      |> Router.call(@options)

    assert conn.status == @http_status_forbidden
    assert conn.resp_body == "Kafka integration disabled"
  end

  test "create stream with wrong prefix" do
    conn =
      :post
      |> conn(create_url("/streams"), %{"stream_pref" => "prefix"})
      |> Router.call(@options)

    assert conn.status == @http_status_bad_request
  end

  test "create stream without prefix" do
    conn =
      :post
      |> conn(create_url("/streams"), %{})
      |> Router.call(@options)

    assert conn.status == @http_status_bad_request
  end

  defp create_url(endpoint) do
    secret = Application.get_env(:ockam_hub, :auth_message)
    "#{endpoint}/?token=#{secret}"
  end
end
