defmodule Ockam.Tests do
  use ExUnit.Case, async: true
  doctest Ockam

  describe "Ockam.start/2" do
    test "Ockam.Router is started", do: Ockam.Router |> find_child |> Process.alive?() |> assert
    test "Ockam.Nodes is started", do: Ockam.Nodes |> find_child |> Process.alive?() |> assert
    test "Ockam.Topics is started", do: Ockam.Topics |> find_child |> Process.alive?() |> assert
  end

  def find_child(name) do
    {_, pid, _, _} =
      Ockam |> Supervisor.which_children() |> Enum.find(fn {t, _, _, _} -> t == name end)

    pid
  end
end
