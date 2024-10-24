defmodule Ockam.Examples.Routing.UDS do
  @moduledoc """
  Example message routing through the UDS transport

  server() - start UDS server with echoer
  client() - establish a connection and send a message to the echoer and wait for response

  """
  alias Ockam.Examples.Echoer
  alias Ockam.Transport.UDS

  require Logger

  def server do
    ## Start a transport with listener on port
    UDS.start(path: "/tmp/server.sock")
    Echoer.create(address: "echoer")
  end

  def client do
    UDS.start(path: "/tmp/client.sock")

    ## Register this process to receive messages
    my_address = "example_run"
    Ockam.Node.register_address(my_address)

    {:ok, server_host_address} = Ockam.Transport.UDS.Client.create(path: "/tmp/server.sock")
    {:ok, hop} = Ockam.Transport.UDS.Client.create(path: "/tmp/client.sock")

    Ockam.Router.route(%{
      onward_route: [server_host_address, hop, "echoer"],
      payload: "Hello localhost!",
      return_route: [my_address]
    })

    receive do
      %{onward_route: [^my_address], return_route: [tcp_client | _], payload: "Hello localhost!"} =
          reply ->
        Logger.info("Received message: #{inspect(reply)}")
        {:ok, tcp_client}
    end
  end
end
