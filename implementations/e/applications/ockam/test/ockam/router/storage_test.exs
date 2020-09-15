defmodule Ockam.Router.Storage.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router.Storage
  alias Ockam.Router.Storage

  describe "Ockam.Router.Storage" do
    test "works" do
      nil = Ockam.Router.Storage.get(:a)
      :ok = Ockam.Router.Storage.put(:a, 100)
      100 = Ockam.Router.Storage.get(:a)
      :ok = Ockam.Router.Storage.delete(:a)
      nil = Ockam.Router.Storage.get(:a)
    end
  end
end
