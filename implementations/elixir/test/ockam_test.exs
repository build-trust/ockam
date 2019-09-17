defmodule OckamTest do
  use ExUnit.Case
  doctest Ockam

  test "random" do
    assert Ockam.random() > 0
  end
end
