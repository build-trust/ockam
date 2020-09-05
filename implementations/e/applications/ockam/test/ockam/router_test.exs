defmodule Ockam.Router.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router
  alias Ockam.Router

  describe "Ockam.Router.register_address_handler/2" do
    test "registered address handler is called" do
      Router.register_address_handler(1, fn _message -> :ok end)
      assert :ok === Router.route(%{onward_route: [1], payload: :test_message})
    end
  end

  describe "Ockam.Router.unregister_address_handler/2" do
    test "unregister_address_handler works" do
      Router.register_address_handler(1, fn _message -> :ok end)
      assert :ok === Router.route(%{onward_route: [1], payload: :test_message})
      Router.unregister_address_handler(1)
      assert {:error, _} = Router.route(%{onward_route: [1], payload: :test_message})
    end
  end

  describe "Ockam.Router.register_address_type_handler/2" do
    test "registered address type handler is called" do
      Router.register_address_type_handler(100, fn _message -> :ok end)
      assert :ok === Router.route(%{onward_route: [{100, 1}], payload: :test_message})
    end
  end

  describe "Ockam.Router.unregister_address_type_handler/2" do
    test "registered address type handler is called" do
      Router.register_address_type_handler(100, fn _message -> :ok end)
      assert :ok === Router.route(%{onward_route: [{100, 1}], payload: :test_message})
      Router.unregister_address_type_handler(100)
      assert {:error, _} = Router.route(%{onward_route: [{100, 1}], payload: :test_message})
    end
  end

  describe "Ockam.Router.register_default_handler/1" do
    test "registered default handler is called" do
      Router.register_default_handler(fn _message -> :ok end)
      assert :ok === Router.route(:test_message)
      assert :ok === Router.route(%{onward_route: [{100, 1}], payload: :test_message})
    end
  end
end
