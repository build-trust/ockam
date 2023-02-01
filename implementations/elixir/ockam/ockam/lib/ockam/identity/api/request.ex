defmodule Ockam.Identity.API.Request do
  @moduledoc """
  Identity API request encoding
  """
  alias Ockam.TypedCBOR

  @validate_identity_change_history {:struct, %{identity: %{key: 1, schema: :binary}}}

  @create_signature {:struct,
                     %{
                       identity: %{key: 1, schema: :binary},
                       state: %{key: 2, schema: :binary},
                       vault_name: %{key: 3, schema: :string}
                     }}

  @verify_signature {:struct,
                     %{
                       identity: %{key: 1, schema: :binary},
                       state: %{key: 2, schema: :binary},
                       proof: %{key: 3, schema: :binary}
                     }}

  @compare_identity_change_history {:struct,
                                    %{
                                      identity: %{key: 1, schema: :binary},
                                      known_identity: %{key: 2, schema: :binary}
                                    }}

  def create() do
    ""
  end

  def validate_identity_change_history(identity) do
    TypedCBOR.encode!(@validate_identity_change_history, %{identity: identity})
  end

  def create_signature(vault_name, identity, auth_hash) do
    TypedCBOR.encode!(@create_signature, %{
      identity: identity,
      state: auth_hash,
      vault_name: vault_name
    })
  end

  def verify_signature(identity, proof, auth_hash) do
    TypedCBOR.encode!(@verify_signature, %{identity: identity, state: auth_hash, proof: proof})
  end

  def compare_identity_change_history(identity, known_identity) do
    TypedCBOR.encode!(@compare_identity_change_history, %{
      identity: identity,
      known_identity: known_identity
    })
  end

  ## For testing purposes

  def decode_create("") do
    {:ok, "", ""}
  end

  def decode_validate_identity_change_history(data) do
    TypedCBOR.decode_strict(@validate_identity_change_history, data)
  end

  def decode_create_signature(data) do
    TypedCBOR.decode_strict(@create_signature, data)
  end

  def decode_verify_signature(data) do
    TypedCBOR.decode_strict(@verify_signature, data)
  end

  def decode_compare_identity_change_history(data) do
    TypedCBOR.decode_strict(@compare_identity_change_history, data)
  end
end
