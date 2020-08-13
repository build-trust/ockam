defmodule Ockam.Message.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Message
  alias Ockam.Message

  describe "Ockam.Message" do
    test "can be created", do: %Message{payload: "hello"}

    test "version is 1 by default" do
      message = %Message{}
      assert 1 === message.version
    end

    test "encode then decode" do
      message = %Message{payload: "hello"}
      decoded = message |> Message.encode() |> Message.decode()
      assert message === decoded
    end

    test "ping encode then decode" do
      message = %Message{payload: :ping}
      decoded = message |> Message.encode() |> Message.decode()
      assert message === decoded
    end

    test "with route encode then decode" do
      message = %Message{
        payload: :ping,
        onward_route: [
          "ff",
          {:tcp, {{1, 1, 1, 1}, 9000}},
          {:udp, {{1, 1, 1, 1}, 9000}},
          {:tcp, {{1, 1, 1, 1, 1, 1, 1, 1}, 9000}},
          {:udp, {{1, 1, 1, 1, 1, 1, 1, 1}, 9000}}
        ]
      }

      decoded = message |> Message.encode() |> Message.decode()
      assert message === decoded
    end
  end
end
