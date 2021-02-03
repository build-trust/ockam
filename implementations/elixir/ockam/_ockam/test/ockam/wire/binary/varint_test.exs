defmodule Ockam.Wire.Binary.VarInt.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Wire.Binary.VarInt
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  doctest VarInt

  @in_range_cases %{
    0 => <<0>>,
    1 => <<1>>,
    16_383 => <<255, 127>>
  }

  @invalid_integers [-1, 16_384]
  @invalid_encoded_binaries [
    <<255, 128>>,
    <<255, 255>>
  ]

  describe "encode/1" do
    test "succeeds on in range inputs" do
      Enum.each(@in_range_cases, fn {k, v} ->
        assert v === VarInt.encode(k)
      end)
    end

    test "fails on out of range inputs" do
      Enum.each(@invalid_integers, fn i ->
        assert {:error, %EncodeError{reason: reason}} = VarInt.encode(i)
        assert {:argument_is_not_an_integer_in_expected_range, metadata} = reason
        assert [expected_range: 0..16_383, argument: i] = metadata
      end)
    end
  end

  describe "decode/1" do
    test "succeeds to decode correctly encoded binaries" do
      Enum.each(@in_range_cases, fn {k, v} ->
        assert VarInt.decode(v) === {k, ""}
      end)
    end

    test "fails to decode incorrectly encoded binaries" do
      Enum.each(@invalid_encoded_binaries, fn b ->
        assert {:error, %DecodeError{reason: _reason}} = VarInt.decode(b)
      end)
    end
  end
end
