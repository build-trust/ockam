defmodule Ockam.Identity.PurposeKeyAttestation do
  @moduledoc false
  defstruct [:attestation]

  @type t() :: %Ockam.Identity.PurposeKeyAttestation{}
end

defimpl CBOR.Encoder, for: Ockam.Identity.PurposeKeyAttestation do
  def encode_into(t, acc) do
    acc <> t.attestation
  end
end
