defmodule Ockam.Transport.UDP.Tests do
  use ExUnit.Case, async: true
  doctest Ockam

  describe "Ockam.Transport.UDP.start/2" do
    test "Ockam.Transport.UDP.Server is started" do
      Ockam.Transport.UDP.Server |> find_child |> Process.alive?() |> assert
    end
  end

  def find_child(name) do
    {_, pid, _, _} =
      Ockam.Transport.UDP |> Supervisor.which_children() |> Enum.find(fn {t, _, _, _} -> t == name end)

    pid
  end
end
