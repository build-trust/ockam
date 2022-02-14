defmodule Ockam.Examples.Routing.TCP do
  @moduledoc """
  Example message routing through the TCP transport

  server() - start TCP server with echoer
  server(port) - start TCP server on port
  client() - establish a connection and send a message to the echoer and wait for response
  client(host, port) - connect to transport on host:port and send a message and wait for response

  send_and_wait(client, message) - send message to existing client connection and wait for response

  """
  alias Ockam.Examples.Echoer
  alias Ockam.Transport.TCP
  alias Ockam.Transport.TCPAddress

  require Logger

  def server(port \\ 4000) do
    ## Start a transport with listener on port
    TCP.start(listen: [port: port])
    Echoer.create(address: "echoer")
  end

  def client(host \\ "localhost", port \\ 4000) do
    ## Start a transport without a listener
    TCP.start()

    server_host_address = TCPAddress.new(host, port)

    ## Register this process to receive messages
    my_address = "example_run"
    Ockam.Node.register_address(my_address)

    Ockam.Router.route(%{
      onward_route: [server_host_address, "echoer"],
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

  def send_and_wait(client, message, return_address \\ "example_run") do
    Ockam.Router.route(%{
      onward_route: [client, "echoer"],
      payload: message,
      return_route: [return_address]
    })

    receive do
      %{onward_route: [^return_address], payload: ^message} = reply ->
        Logger.info("Received message: #{inspect(reply)}")
        :ok
    end
  end
end
