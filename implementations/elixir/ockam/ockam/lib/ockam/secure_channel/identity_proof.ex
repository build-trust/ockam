defmodule Ockam.SecureChannel.IdentityProof do
  @moduledoc """
  Identity Proof and Credentials exchanged
  during secure channel handshake
  """

  alias __MODULE__
  @bare_struct {:struct, [contact: :data, signature: :data, credentials: {:array, :data}]}
  defstruct [:contact, :signature, :credentials]

  def encode(%IdentityProof{} = p) do
    :bare.encode(p, @bare_struct)
  end

  def decode(encoded) do
    case :bare.decode(encoded, @bare_struct) do
      {:ok, map, ""} ->
        {:ok, struct(IdentityProof, map)}

      error ->
        {:error, {:invalid_identity_proof_msg, error}}
    end
  end
end
