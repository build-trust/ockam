defmodule Ockam.Credential.Verifier.Stub do
  @moduledoc """
  Stub implementation for credential verifier
  """
  alias Ockam.Credential.AttributeSet

  def verify(credential, subject_id, authorities)
      when is_binary(credential) and is_binary(subject_id) and is_map(authorities) do
    with {:ok, %{attributes: attributes, expiration: expiration}} <- parse_credential(credential) do
      {:ok, %AttributeSet{attributes: attributes, expiration: expiration}}
    end
  end

  def make_credential(attributes, expiration) when is_map(attributes) do
    CBOR.encode(%{attributes: attributes, expiration: expiration})
  end

  def parse_credential(credential) do
    case CBOR.decode(credential) do
      {:ok, %{"attributes" => attributes, "expiration" => expiration}, ""}
      when is_map(attributes) ->
        {:ok, %{attributes: attributes, expiration: expiration}}

      _other ->
        {:error, :invalid_credential}
    end
  end

  def expiration() do
    now = System.os_time(:second)
    now + 3600
  end
end
