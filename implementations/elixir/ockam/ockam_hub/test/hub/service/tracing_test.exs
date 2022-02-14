defmodule Test.Hub.Service.TracingTest do
  use ExUnit.Case

  test "trace payloads" do
    Ockam.Hub.Service.Tracing.create(address: "tracing_service")
    Ockam.Hub.Service.Echo.create(address: "echo_service")
    Ockam.Node.register_address("TEST")

    Ockam.Router.route(%{
      onward_route: ["tracing_service"],
      return_route: ["TEST"],
      payload: "register"
    })

    tracing_address =
      receive do
        %{payload: "register", return_route: [tracing_address]} -> tracing_address
      after
        5000 ->
          exit("Cannot register tracing address")
      end

    payload = "Hello!"
    echo_request = %{onward_route: [tracing_address, "echo_service"], payload: payload}
    Ockam.Workers.Call.call(echo_request)

    # Receive outgoing message
    receive do
      %{payload: ^payload} -> :ok
    after
      5000 ->
        exit("Timeout receiving trace message")
    end

    # Receive reply message
    receive do
      %{payload: ^payload} -> :ok
    after
      5000 ->
        exit("Timeout receiving trace message")
    end
  end
end
