defmodule Ockam.SecureChannel.IdentityProof do
  @moduledoc """
  Identity Proof and Credentials exchanged
  during secure channel handshake
  """
alias Ockam.SecureChannel.IdentityProof

  defstruct [:contact, :attestation, :credentials]


  def encode(t), do:  CBOR.encode(t)
  def decode(data) do
    case CBOR.decode(data) do
      {:ok, %{1 => change_history, 2 => attestation, 3=> credentials}, ""} ->
        {:ok, %IdentityProof{contact: CBOR.encode(change_history), attestation: CBOR.encode(attestation), credentials: credentials}}
      {:ok, decoded, rest} -> {:error, {:decode_error, {:extra_data, rest, decoded}, data}}
      {:error, _reason} = error -> error
    end
  end

end


defimpl CBOR.Encoder, for: Ockam.SecureChannel.IdentityProof  do
  def encode_into(t, acc) do
    %{1 => t.contact,
      2 => t.attestation,
      3 => []} |> CBOR.Encoder.encode_into(acc)
  end
end



  """
  pub(super) struct IdentityAndCredentials {
    /// Exported identity
    #[n(1)] pub(super) change_history: ChangeHistory,
    /// The Purpose Key guarantees that the other end has access to the private key of the identity
    /// The Purpose Key here is also the static key of the noise ('x') and is issued with the static
    /// key of the identity
    #[n(2)] pub(super) purpose_key_attestation: PurposeKeyAttestation,
    /// Credentials associated to the identity along with corresponding Credentials Purpose Keys
    /// to verify those Credentials
    #[n(3)] pub(super) credentials: Vec<CredentialAndPurposeKey>,
  }
  """
