defmodule Ockam.Channel.Protocol.Test do
  use ExUnit.Case, async: true

  alias Ockam.Channel.Protocol

  test "round-trip protocol" do
    expected = "Noise_XX_25519_AESGCM_SHA256"
    assert {:ok, p} = Protocol.from_name(expected)
    assert ^expected = Protocol.name(p)
  end
end
