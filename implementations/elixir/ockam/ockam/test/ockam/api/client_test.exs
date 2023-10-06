defmodule Ockam.API.Tests.EchoAPI do
  @moduledoc false
  use Ockam.Worker

  alias Ockam.API.Request
  alias Ockam.API.Response

  alias Ockam.Worker

  @impl true
  def handle_message(message, state) do
    {:ok, request} = Request.from_message(message)
    response = Response.reply_to(request, 200, request.body)
    Worker.route(Response.to_message(response, [state.address]), state)
    {:ok, state}
  end
end

defmodule Ockam.API.Client.Tests do
  use ExUnit.Case, async: true

  alias Ockam.API.Client

  test "Request timeout" do
    {:error, :timeout} = Client.sync_request(:get, "foo", "HI", ["non_existent_api"], 100)
  end

  test "Send request message" do
    {:ok, _echoer} = Ockam.API.Tests.EchoAPI.create(address: "echo_api")
    {:ok, self_address} = Ockam.Node.register_random_address()

    on_exit(fn ->
      Ockam.Node.stop("echo_api")
      Ockam.Node.unregister_address(self_address)
    end)

    ## Fail because echoer does not implement API, but that's OK
    body = "HELLO"

    {:ok, response} = Client.sync_request(:get, "foo", body, ["echo_api"], 1000, self_address)

    assert response.body == body
    assert response.status == 200

    assert response.to_route == [self_address]
  end
end
