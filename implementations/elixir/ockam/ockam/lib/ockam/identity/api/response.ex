defmodule Ockam.Identity.API.Response do
  @moduledoc """
  Identity API response decoding
  """
  alias Ockam.TypedCBOR

  @create {:struct,
           %{identity: %{key: 1, schema: :binary}, identity_id: %{key: 2, schema: :string}}}

  @validate_identity_change_history {:struct, %{identity_id: %{key: 1, schema: :string}}}

  @create_signature {:struct, %{proof: %{key: 1, schema: :binary}}}

  @verify_signature {:struct, %{verified: %{key: 1, schema: :boolean}}}

  @compare_identity_change_history {:enum, [none: 0, equal: 1, conflict: 2, newer: 3, older: 4]}

  def create(data) do
    TypedCBOR.decode_strict(@create, data)
  end

  def validate_identity_change_history(data) do
    TypedCBOR.decode_strict(@validate_identity_change_history, data)
  end

  def create_signature(data) do
    TypedCBOR.decode_strict(@create_signature, data)
  end

  def verify_signature(data) do
    TypedCBOR.decode_strict(@verify_signature, data)
  end

  def compare_identity_change_history(data) do
    TypedCBOR.decode_strict(@compare_identity_change_history, data)
  end

  ## For testing purposes

  def encode_create(decoded) do
    TypedCBOR.encode(@create, decoded)
  end

  def encode_validate_identity_change_history(decoded) do
    TypedCBOR.encode(@validate_identity_change_history, decoded)
  end

  def encode_create_signature(decoded) do
    TypedCBOR.encode(@create_signature, decoded)
  end

  def encode_verify_signature(decoded) do
    TypedCBOR.encode(@verify_signature, decoded)
  end

  def encode_compare_identity_change_history(decoded) do
    TypedCBOR.encode(@compare_identity_change_history, decoded)
  end
end
