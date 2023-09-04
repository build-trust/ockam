defmodule Ockam.Identity.PurposeKeyAttestation do
  defstruct [:attestation]
end

defimpl CBOR.Encoder, for: Ockam.Identity.PurposeKeyAttestation do
  def encode_into(t, acc) do
    acc <> t.attestation
  end
end
