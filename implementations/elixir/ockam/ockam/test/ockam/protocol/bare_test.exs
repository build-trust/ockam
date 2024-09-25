defmodule Ockam.Bare.Extended.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Bare.Extended
  alias Ockam.Bare.Extended

  def encode_decode_test(data, schema, encoded) do
    actual_encoded = Extended.encode(data, schema)
    assert actual_encoded == encoded
    {:ok, actual_data, <<>>} = Extended.decode(encoded, schema)
    assert actual_data == data
  end

  describe "Bare tuple" do
    test "Bare tuples can be encoded and decoded" do
      encode_decode_test(
        [0xAA, 0xBB, 0xCC],
        {:tuple, [:int, :int, :int]},
        <<212, 2, 246, 2, 152, 3>>
      )

      encode_decode_test([0xAA, 0xBB], {:tuple, [:int, :int]}, <<212, 2, 246, 2>>)
      encode_decode_test([0xAA], {:tuple, [:int]}, <<212, 2>>)
    end

    test "Bare tuple with optional elements" do
      encode_decode_test(
        [0xAA, 0xBB, 0xCC],
        {:tuple, [:int, :int, {:optional, :int}]},
        <<212, 2, 246, 2, 1, 152, 3>>
      )

      encode_decode_test(
        [0xAA, 0xBB, :undefined],
        {:tuple, [:int, :int, {:optional, :int}]},
        <<212, 2, 246, 2, 0>>
      )

      encode_decode_test([0xAA, 0xBB], {:tuple, [:int, {:optional, :int}]}, <<212, 2, 1, 246, 2>>)
      encode_decode_test([0xAA, :undefined], {:tuple, [:int, {:optional, :int}]}, <<212, 2, 0>>)

      encode_decode_test([0xAA], {:tuple, [{:optional, :int}]}, <<1, 212, 2>>)
      encode_decode_test([:undefined], {:tuple, [{:optional, :int}]}, <<0>>)
    end
  end
end
