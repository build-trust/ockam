defmodule Ockam.Examples.Messaging.Ordering do
  @moduledoc """
  Examples of using ordering pipes

  Creates a shuffle worker to re-order messages
  Sends messages through shuffle and through shuffle wrapped in an ordered pipe
  """

  alias Ockam.Examples.Messaging.Shuffle

  def check_strict(pipe_mod) do
    Ockam.Node.register_address("app")

    {:ok, shuffle} = Shuffle.create([])

    {:ok, receiver} = pipe_mod.receiver().create([])

    {:ok, sender} = pipe_mod.sender().create(receiver_route: [shuffle, receiver])

    Enum.each(1..100, fn n ->
      Ockam.Router.route(%{
        onward_route: [sender, "app"],
        return_route: ["ordered"],
        payload: "#{n}"
      })

      Ockam.Router.route(%{
        onward_route: [shuffle, "app"],
        return_route: ["unordered"],
        payload: "#{n}"
      })
    end)

    ## receive 100 messages
    unordered =
      Enum.map(1..100, fn _n ->
        receive do
          %{payload: pl, return_route: ["unordered"]} -> String.to_integer(pl)
        end
      end)

    ordered =
      Enum.map(1..100, fn _n ->
        receive do
          %{payload: pl} -> String.to_integer(pl)
        end
      end)

    {unordered, ordered}
    # payloads == Enum.sort(payloads)
  end

  def check_monotonic(pipe_mod) do
    Ockam.Node.register_address("app")

    {:ok, shuffle} = Shuffle.create([])

    {:ok, receiver} = pipe_mod.receiver().create([])

    {:ok, sender} = pipe_mod.sender().create(receiver_route: [shuffle, receiver])

    Enum.each(1..100, fn n ->
      Ockam.Router.route(%{
        onward_route: [sender, "app"],
        return_route: ["ordered"],
        payload: "#{n}"
      })

      Ockam.Router.route(%{
        onward_route: [shuffle, "app"],
        return_route: ["unordered"],
        payload: "#{n}"
      })
    end)

    ## receive 100 messages
    unordered =
      1..100
      |> Enum.map(fn _n ->
        receive do
          %{payload: pl, return_route: ["unordered"]} -> String.to_integer(pl)
        after
          100 ->
            nil
        end
      end)
      |> Enum.reject(&is_nil/1)

    ordered =
      1..100
      |> Enum.map(fn _n ->
        receive do
          %{payload: pl, return_route: rr} when rr != ["unordered"] -> String.to_integer(pl)
        after
          100 ->
            nil
        end
      end)
      |> Enum.reject(&is_nil/1)

    {unordered, ordered}
    # payloads == Enum.sort(payloads)
  end
end
