defmodule Ockam.API.Response.Tests do
  @moduledoc false
  use ExUnit.Case, async: true

  alias Ockam.API.Request
  alias Ockam.API.Response

  alias Ockam.Message

  test "Encode/decode" do
    ## Only these 4 firelds are encoded
    response = %Response{
      id: Response.gen_id(),
      request_id: Request.gen_id(),
      status: 200,
      body: "something"
    }

    encoded = Response.encode(response)
    {:ok, decoded} = Response.decode(encoded)
    assert response == decoded
  end

  test "To/from message" do
    ## to_route is used as a message onward_route
    response = %Response{
      id: Response.gen_id(),
      request_id: Request.gen_id(),
      status: 200,
      body: "something",
      to_route: ["onward", "route"]
    }

    message = Response.to_message(response, ["return", "route"])

    assert Message.onward_route(message) == response.to_route

    {:ok, response_from} = Response.from_message(message)

    ## from_route is a return route from the message
    assert Message.return_route(message) == response_from.from_route

    assert Map.take(response_from, [:id, :request_id, :status, :body, :to_route]) ==
             Map.take(response, [:id, :request_id, :status, :body, :to_route])
  end

  test "reply to request" do
    request = %Request{id: Request.gen_id(), from_route: ["from", "route"]}

    response = Response.reply_to(request, 300, "response_body")

    assert response.request_id == request.id
    assert response.to_route == request.from_route
  end
end
