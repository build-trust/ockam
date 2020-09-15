defmodule Ockam.Router.Storage.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router.Storage
  alias Ockam.Router.Storage

  describe "Ockam.Router.Storage" do
    test "works" do
      nil = Storage.get(:a)
      :ok = Storage.put(:a, 100)
      100 = Storage.get(:a)
      :ok = Storage.delete(:a)
      nil = Storage.get(:a)
    end
  end
end
