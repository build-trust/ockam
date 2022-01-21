defmodule Ockam.Tests do
  use ExUnit.Case, async: true
  doctest Ockam

  describe "Ockam.start/2" do
    test "Ockam.Router is started", do: Ockam.Router |> find_child |> Process.alive?() |> assert
    test "Ockam.Node is started", do: Ockam.Node |> find_child |> Process.alive?() |> assert
  end

  def find_child(name) do
    {_id, pid, _type, _modules} =
      Ockam
      |> Supervisor.which_children()
      |> Enum.find(fn {t, _child, _type, _modules} -> t == name end)

    pid
  end
end
