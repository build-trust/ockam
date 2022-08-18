defmodule Ockam.Credential.Authorization do
  @moduledoc """
  Worker authorization helpers for attribute based access control
  Using AttributeStorageETS as an attribute storage
  """

  alias Ockam.Credential.AttributeStorageETS
  alias Ockam.Message

  def match_attributes(%Message{} = message, required_attributes) do
    identity_id = Message.local_metadata_value(message, :identity_id)
    match_attributes(identity_id, required_attributes)
  end

  def match_attributes(identity_id, required_attributes) do
    required_attributes = normalize_attributes(required_attributes)

    identity_attributes =
      AttributeStorageETS.get_attributes(identity_id)
      |> normalize_attributes()

    Enum.reduce(required_attributes, :ok, fn
      {key, value}, :ok ->
        case Map.get(identity_attributes, key) do
          ^value -> :ok
          nil -> {:error, {:attribute_missing, key}}
          other -> {:error, {:attribute_mismatch, key, value, other}}
        end

      _kv, error ->
        error
    end)
  end

  defp normalize_attributes(attributes) do
    Enum.map(
      attributes,
      fn {key, val} ->
        {to_string(key), to_string(val)}
      end
    )
    |> Map.new()
  end
end
