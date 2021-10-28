defmodule Ockam.Examples.Delivery do
  @moduledoc """
  Examples of using delivery pipe

  Creates a filter worker to lose messages
  Sends messages through filter and through filter wrapped in a delivery pipe

  Returns a sequence of messages received with unreliable delivety (through filter)
  and reliable delivery (through pipe)
  """

  alias Ockam.Examples.Messaging.Filter

  def run() do
    pipe_mod = Ockam.Messaging.Delivery.ResendPipe

    {:ok, filter} = Filter.create([])
    {:ok, receiver} = pipe_mod.receiver().create([])
    ## Local delivery is fast, set lower confirm timeout
    {:ok, sender} =
      pipe_mod.sender().create(receiver_route: [filter, receiver], confirm_timeout: 200)

    {:ok, me} = Ockam.Node.register_random_address()

    ## Seng 100 messages to self through filter

    Enum.each(1..100, fn n ->
      Ockam.Router.route(%{
        onward_route: [filter, me],
        return_route: [me],
        payload: "unreliable #{n}"
      })
    end)

    unreliable =
      1..100
      |> Enum.map(fn n ->
        expected_payload = "unreliable #{n}"

        receive do
          %{onward_route: [^me], payload: ^expected_payload} ->
            expected_payload
        after
          200 ->
            nil
        end
      end)
      |> Enum.reject(&is_nil/1)

    ## Send 100 messages to self through reliable pipe

    Enum.each(1..100, fn n ->
      Ockam.Router.route(%{
        onward_route: [sender, me],
        return_route: [me],
        payload: "reliable #{n}"
      })
    end)

    ## Wait for message #100

    expected_100 = "reliable 100"

    receive do
      %{onward_route: [^me], payload: ^expected_100} ->
        :ok
        ## Fail in 10 minutes
    after
      600_000 ->
        raise "Message #100 was not received in 10 minutes"
    end

    reliable = receive_all()

    %{unreliable: unreliable, reliable: reliable}
  end

  def receive_all(msgs \\ []) do
    receive do
      %{payload: pl} ->
        receive_all(msgs ++ [pl])
    after
      0 ->
        msgs
    end
  end
end
