defmodule Ockam.Messaging.Delivery.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Messaging.Delivery.Tests.Filter

  delivery_pipes = [Ockam.Messaging.Delivery.ResendPipe]

  Enum.each(delivery_pipes, fn pipe ->
    test "Pipe #{pipe} is delivers all messages" do
      pipe_mod = unquote(pipe)

      {:ok, me} = Ockam.Node.register_random_address()

      {:ok, filter} = Filter.create([])

      {:ok, re_receiver} = pipe_mod.receiver().create([])

      {:ok, re_sender} =
        pipe_mod.sender().create(receiver_route: [filter, re_receiver], confirm_timeout: 50)

      Enum.each(1..100, fn n ->
        Ockam.Router.route(%{
          onward_route: [re_sender, me],
          return_route: [me],
          payload: "HI #{n}!"
        })
      end)

      Enum.each(1..100, fn n ->
        expected_payload = "HI #{n}!"

        receive do
          %{onward_route: [^me], payload: ^expected_payload} ->
            :ok
            ## TODO: optimize test run time
        after
          60_000 ->
            raise "Message not delivered #{n}"
        end
      end)
    end
  end)

  Enum.each(delivery_pipes, fn pipe ->
    test "Pipe #{pipe} can be deduplicated with indexed pipe" do
      pipe_mod = unquote(pipe)

      index_pipe_mod = Ockam.Messaging.Ordering.Strict.IndexPipe

      {:ok, me} = Ockam.Node.register_random_address()

      {:ok, filter} = Filter.create([])

      {:ok, re_receiver} = pipe_mod.receiver().create([])

      {:ok, re_sender} =
        pipe_mod.sender().create(receiver_route: [filter, re_receiver], confirm_timeout: 50)

      {:ok, ord_receiver} = index_pipe_mod.receiver().create([])

      {:ok, ord_sender} =
        index_pipe_mod.sender().create(receiver_route: [re_sender, ord_receiver])

      Enum.each(1..100, fn n ->
        Ockam.Router.route(%{
          onward_route: [ord_sender, me],
          return_route: [me],
          payload: "HI #{n}!"
        })
      end)

      Enum.each(1..100, fn n ->
        expected_payload = "HI #{n}!"

        receive do
          %{onward_route: [^me], payload: ^expected_payload} ->
            :ok
            ## TODO: optimize test run time
        after
          60_000 ->
            raise "Message not delivered #{n}"
        end
      end)

      assert [] == collect_rest_msgs(me)
    end
  end)

  def collect_rest_msgs(me, msgs \\ [], timeout \\ 5_000) do
    receive do
      %{onward_route: [^me], payload: payload} ->
        collect_rest_msgs(me, [payload | msgs], timeout)
    after
      timeout ->
        msgs
    end
  end
end
