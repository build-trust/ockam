defmodule Ockam.Messaging.Ordering.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Messaging.Ordering.Tests.Shuffle

  monotonic_pipes = [Ockam.Messaging.Ordering.Monotonic.IndexPipe]

  continuous_pipes = [
    Ockam.Messaging.Ordering.Strict.ConfirmPipe,
    Ockam.Messaging.Ordering.Strict.IndexPipe
  ]

  Enum.each(continuous_pipes, fn pipe ->
    test "Pipe #{pipe} is continuously ordered" do
      pipe_mod = unquote(pipe)
      {:ok, me} = Ockam.Node.register_random_address()

      {:ok, shuffle} = Shuffle.create([])

      {:ok, receiver} = pipe_mod.receiver().create([])

      {:ok, sender} = pipe_mod.sender().create(receiver_route: [shuffle, receiver])

      Enum.each(1..100, fn n ->
        Ockam.Router.route(%{
          onward_route: [sender, me],
          return_route: ["ordered"],
          payload: "#{n}"
        })
      end)

      ## receive 100 messages
      ordered =
        Enum.map(1..100, fn n ->
          receive do
            %{payload: pl} -> String.to_integer(pl)
          after
            2000 ->
              raise "message #{n} not received"
          end
        end)

      assert Enum.sort(ordered) == ordered
    end
  end)

  Enum.each(monotonic_pipes, fn pipe ->
    test "Pipe #{pipe} is monotonically ordered" do
      pipe_mod = unquote(pipe)
      {:ok, me} = Ockam.Node.register_random_address()

      {:ok, shuffle} = Shuffle.create([])

      {:ok, receiver} = pipe_mod.receiver().create([])

      {:ok, sender} = pipe_mod.sender().create(receiver_route: [shuffle, receiver])

      Enum.each(1..100, fn n ->
        Ockam.Router.route(%{
          onward_route: [sender, me],
          return_route: ["ordered"],
          payload: "#{n}"
        })
      end)

      ## receive 100 messages with timeout
      ordered =
        1..100
        |> Enum.map(fn _n ->
          receive do
            %{payload: pl} ->
              String.to_integer(pl)
              ## TODO: optimize test run time
          after
            100 ->
              nil
          end
        end)
        |> Enum.reject(&is_nil/1)

      assert Enum.sort(ordered) == ordered
    end
  end)
end
