defmodule Test.Services.ProxyTest do
  use ExUnit.Case

  alias Ockam.Message
  alias Ockam.Router

  alias Ockam.Transport.TCPAddress

  alias Ockam.Services.Echo, as: EchoService
  alias Ockam.Services.Proxy

  alias Ockam.Workers.Call

  alias Test.Utils

  @tcp_port 5000

  ## Helper function to count TCP clients
  def tcp_clients() do
    Ockam.Node.list_addresses()
    |> Enum.filter(fn address ->
      String.starts_with?(address, "TCP_C_") and not String.starts_with?(address, "TCP_C_R")
    end)
  end

  test "echo proxy" do
    {:ok, echo_address} = EchoService.create([])

    on_exit(fn ->
      Ockam.Node.stop(echo_address)
    end)

    forward_route = [echo_address]

    {:ok, proxy_address} = Proxy.create(forward_route: forward_route)

    on_exit(fn ->
      Ockam.Node.stop(proxy_address)
    end)

    my_address = "test_me"
    {:ok, my_address} = Ockam.Node.register_random_address()

    Router.route("Hi echo proxy!", [proxy_address], [my_address])

    assert_receive %Message{
                     onward_route: [my_address],
                     return_route: [proxy_address],
                     payload: "Hi echo proxy!"
                   },
                   2000
  end

  test "echo proxy over tcp" do
    {:ok, echo_address} = EchoService.create([])

    on_exit(fn ->
      Ockam.Node.stop(echo_address)
    end)

    {:ok, listener} = Ockam.Transport.TCP.start(listen: [port: @tcp_port])

    tcp_clients_count = Enum.count(tcp_clients())

    forward_route = [TCPAddress.new("localhost", @tcp_port), echo_address]

    {:ok, proxy_address} = Proxy.create(forward_route: forward_route)

    on_exit(fn ->
      Ockam.Node.stop(proxy_address)
    end)

    {:ok, my_address} = Ockam.Node.register_random_address()

    Router.route("Hi echo proxy!", [proxy_address], [my_address])

    assert_receive %Message{
                     onward_route: [my_address],
                     return_route: [proxy_address],
                     payload: "Hi echo proxy!"
                   },
                   2000

    assert Enum.count(tcp_clients()) == tcp_clients_count + 1

    ## Make sure we don't leak the TCP connections
    Router.route("Hi echo proxy take2!", [proxy_address], [my_address])

    assert_receive %Message{
                     onward_route: [my_address],
                     return_route: [proxy_address],
                     payload: "Hi echo proxy take2!"
                   },
                   2000

    assert Enum.count(tcp_clients()) == tcp_clients_count + 1
  end

  test "proxy provider" do
    System.put_env("SERVICE_PROXY_remote_echo", "1#localhost:4000;0#echo")
    [spec] = Ockam.Services.Provider.Proxy.child_spec(:proxy, [])

    assert %{
             id: :proxy_remote_echo,
             start:
               {Ockam.Services.Proxy, :start_link,
                [[address: "remote_echo", forward_route: "1#localhost:4000;0#echo"]]}
           } = spec
  end
end
