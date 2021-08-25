defmodule Ockam.Examples.Transport.TCP do
  @moduledoc """
  Example usage of tcp transport
  """
  alias Ockam.Transport.TCP
  alias Ockam.Transport.TCPAddress

  def server() do
    ## Start a transport with listener on port 4000
    TCP.start(listen: [port: 4000])
    Ockam.Examples.Transport.Printer.create(address: "printer")
  end

  def client() do
    ## Start a transport without a listener
    TCP.start()

    server_host_address = TCPAddress.new("localhost", 4000)

    Ockam.Router.route(%{
      onward_route: [server_host_address, "printer"],
      payload: "Hello!",
      return_route: []
    })

    server_ip_address = TCPAddress.new({127, 0, 0, 1}, 4000)

    Ockam.Router.route(%{
      onward_route: [server_ip_address, "printer"],
      payload: "Hello!",
      return_route: []
    })

    server_string_ip_address = TCPAddress.new("127.0.0.1", 4000)

    Ockam.Router.route(%{
      onward_route: [server_string_ip_address, "printer"],
      payload: "Hello!",
      return_route: []
    })
  end
end
