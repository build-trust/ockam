defmodule Ockam.Transport.UDP.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.UDP
  # alias Ockam.Transport.UDP
  #
  # setup_all do
  #   nodes = TestCluster.create_nodes("ttt", 1, files: [__ENV__.file])
  #   on_exit(fn -> :ok = TestCluster.destroy_nodes(nodes) end)
  #
  #   test_node = List.first(nodes)
  #   test_node_ip = {127, 0, 0, 1}
  #   test_node_port = 8000
  #
  #   {:ok, _, _} =
  #     run_at(test_node, fn ->
  #       UDP.start_link(%{address: "aa", ip: test_node_ip, port: test_node_port})
  #     end)
  #
  #   {:ok, _, _} = UDP.start_link(%{address: "bb", ip: {127, 0, 0, 1}, port: 9000})
  #
  #   {:ok, %{test_node: {test_node, {test_node_ip, test_node_port}}}}
  # end
  #
  # describe "Ockam.Transport.UDP" do
  #   test "can receive incoming" do
  #     Ockam.Router.register("ffff", self())
  #
  #     Ockam.Router.route(%Ockam.Message{
  #       payload: :ping,
  #       onward_route: [{:udp, {{127, 0, 0, 1}, 8000}}],
  #       return_route: ["ffff"]
  #     })
  #
  #     assert_receive %Ockam.Message{
  #       payload: :pong
  #     }
  #   end
  #
  #   test "raw incoming" do
  #     Ockam.Router.register("ffff", self())
  #
  #     {:ok, _, _} =
  #       UDP.start_link(%{
  #         address: "cc",
  #         ip: {127, 0, 0, 1},
  #         port: 9001,
  #         route_incoming_to: "ffff",
  #         route_external: false,
  #         encode_decode: false
  #       })
  #
  #     open_options = [:binary, :inet, {:ip, {127, 0, 0, 1}}, {:active, true}]
  #     {:ok, socket} = :gen_udp.open(6666, open_options)
  #     :gen_udp.send(socket, {127, 0, 0, 1}, 9001, "hello")
  #
  #     assert_receive %Ockam.Message{
  #       payload: "hello",
  #       onward_route: ["ffff"],
  #       return_route: [udp: {{127, 0, 0, 1}, 6666}]
  #     }
  #   end
  #
  #   test "raw outgoing" do
  #     open_options = [:binary, :inet, {:ip, {127, 0, 0, 1}}, {:active, true}]
  #     {:ok, _socket} = :gen_udp.open(7777, open_options)
  #
  #     {:ok, _, _} =
  #       UDP.start_link(%{
  #         address: "dd",
  #         ip: {127, 0, 0, 1},
  #         port: 9005,
  #         route_external: false,
  #         encode_decode: false
  #       })
  #
  #     "dd"
  #     |> Ockam.Router.whereis()
  #     |> Kernel.send(
  #       {{{127, 0, 0, 1}, 7777},
  #        %Ockam.Message{
  #          payload: "HHHHHHHHH"
  #        }}
  #     )
  #
  #     assert_receive {:udp, _, _, _, "HHHHHHHHH"}
  #   end
  # end
  #
  # def run_at(n, fun) do
  #   caller = self()
  #
  #   Node.spawn(n, fn ->
  #     response = fun.()
  #     send(caller, response)
  #   end)
  #
  #   receive do
  #     r -> r
  #   end
  # end
end
