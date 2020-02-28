defmodule Ockam.Test.Fixtures do
  def curve25519_data() do
    %{
      a_priv:
        Base.decode16!("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
          case: :lower
        ),
      a_pub:
        Base.decode16!("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a",
          case: :lower
        ),
      b_priv:
        Base.decode16!("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb",
          case: :lower
        ),
      b_pub:
        Base.decode16!("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f",
          case: :lower
        ),
      shared:
        Base.decode16!("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742",
          case: :lower
        )
    }
  end

  def blake2b_hmac_data() do
    [
      %{
        key:
          Base.decode16!(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" <>
              "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
          ),
        data:
          Base.decode16!(
            "6666666666666666666666666666666666666666666666666666666666666666" <>
              "6666666666666666666666666666666666666666666666666666666666666666" <>
              "6666666666666666666666666666666666666666666666666666666666666666" <>
              "6666666666666666666666666666666666666666666666666666666666666666"
          ),
        hmac:
          Base.decode16!(
            "4054489AA4225A07BD7F4C89330AA6412B612AADC8FA86AFBC8EC6AC2D0F3AC8" <>
              "ECDB6601B060F47488D4074C562F848B9F6168BA8CDEE22E399057B5D53129C9"
          )
      },
      %{
        key:
          Base.decode16!(
            "4054489AA4225A07BD7F4C89330AA6412B612AADC8FA86AFBC8EC6AC2D0F3AC8" <>
              "ECDB6601B060F47488D4074C562F848B9F6168BA8CDEE22E399057B5D53129C9"
          ),
        data: Base.decode16!("01"),
        hmac:
          Base.decode16!(
            "359D3AA619DF4F73E4E8EA31D05F5631C96F119D46F6BB44B5C7772B862747E7" <>
              "818D4BC8907C1EBA90B06AD7925EC5E751E4E92D0E0233F893CD3FED8DD6FB76"
          )
      },
      %{
        key:
          Base.decode16!(
            "4054489AA4225A07BD7F4C89330AA6412B612AADC8FA86AFBC8EC6AC2D0F3AC8" <>
              "ECDB6601B060F47488D4074C562F848B9F6168BA8CDEE22E399057B5D53129C9"
          ),
        data:
          Base.decode16!(
            "359D3AA619DF4F73E4E8EA31D05F5631C96F119D46F6BB44B5C7772B862747E7" <>
              "818D4BC8907C1EBA90B06AD7925EC5E751E4E92D0E0233F893CD3FED8DD6FB7602"
          ),
        hmac:
          Base.decode16!(
            "37E23F26F8445E3B5A88949B98606131774BA4D15F2C6E17A0A43972BB4EB6B5" <>
              "CBB42F57D8B1B63B4C9EA64B0493E82A6F6D3A7037C33212EF6E4F56E321D4D9"
          )
      }
    ]
  end

  def vectors() do
    %{vectors: vectors} =
      Path.join([__DIR__, "vectors.txt"])
      |> File.read!()
      |> Jason.decode!(keys: :atoms)

    vectors
  end
end
