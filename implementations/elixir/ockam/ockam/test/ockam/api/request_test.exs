defmodule Ockam.API.Request.Tests do
  use ExUnit.Case, async: true

  alias Ockam.API.Request

  alias Ockam.Message

  test "Encode/decode" do
    ## Only these 4 firelds are encoded
    request = %Request{id: Request.gen_id(), path: "my_path", method: :get, body: "something"}
    encoded = Request.encode(request)
    {:ok, decoded} = Request.decode(encoded)
    assert request == decoded
  end

  test "To/from message" do
    ## to_route is used as a message onward_route
    request = %Request{
      id: Request.gen_id(),
      path: "my_path",
      method: :get,
      body: "something",
      to_route: ["onward", "route"]
    }

    message = Request.to_message(request, ["return", "route"])

    assert Message.onward_route(message) == request.to_route

    {:ok, request_from} = Request.from_message(message)

    ## from_route is a return route from the message
    assert Message.return_route(message) == request_from.from_route

    assert Map.take(request_from, [:id, :path, :method, :body, :to_route]) ==
             Map.take(request, [:id, :path, :method, :body, :to_route])
  end
end
