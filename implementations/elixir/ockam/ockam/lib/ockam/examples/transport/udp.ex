defmodule Ockam.Examples.Transport.UDP do
  @moduledoc """
  An example module on how to use the UDP transport
  """
  alias Ockam.Transport.UDP
  alias Ockam.Transport.UDPAddress

  require Logger

  def alice() do
    ## Start a transport to use a port 4000
    UDP.start(port: 4000)
    Ockam.Examples.Transport.Printer.create(address: "printer")
  end

  def bob() do
    ## Start a transport to use a port 3000
    UDP.start(port: 3000)

    server_ip_address = UDPAddress.new({127, 0, 0, 1}, 4000)

    Ockam.Router.route(%{
      onward_route: [server_ip_address, "printer"],
      payload: "Hello!",
      return_route: []
    })

    server_string_ip_address = UDPAddress.new("127.0.0.1", 4000)

    Ockam.Router.route(%{
      onward_route: [server_string_ip_address, "printer"],
      payload: "Hello!",
      return_route: []
    })
  end
end
