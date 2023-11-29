defmodule Ockam.SecureChannel.IdentityProof do
  @moduledoc """
  Identity Proof and Credentials exchanged
  during secure channel handshake
  """
  alias Ockam.SecureChannel.IdentityProof

  defstruct [:contact, :attestation, :credentials]

  def encode(t), do: CBOR.encode(t)

  def decode(data) do
    case CBOR.decode(data) do
      {:ok, [change_history, attestation, credentials], ""} ->
        {:ok,
         %IdentityProof{
           contact: CBOR.encode(change_history),
           attestation: CBOR.encode(attestation),
           credentials: Enum.map(credentials, fn c -> CBOR.encode(c) end)
         }}

      {:ok, decoded, rest} ->
        {:error, {:decode_error, {:extra_data, rest, decoded}, data}}

      {:error, _reason} = error ->
        error
    end
  end
end

defmodule Ockam.SecureChannel.IdentityProof.Credential do
  @moduledoc false
  defstruct [:data]
end

defimpl CBOR.Encoder, for: Ockam.SecureChannel.IdentityProof.Credential do
  def encode_into(t, acc), do: acc <> t.data
end

defimpl CBOR.Encoder, for: Ockam.SecureChannel.IdentityProof do
  def encode_into(t, acc) do
    [
      t.contact,
      t.attestation,
      Enum.map(t.credentials, fn c -> %Ockam.SecureChannel.IdentityProof.Credential{data: c} end)
    ]
    |> CBOR.Encoder.encode_into(acc)
  end
end
