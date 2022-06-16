defmodule Ockam.Identity.Stub do
  @moduledoc """
  Stub for `Ockam.Identity`
  """

  @type t() :: binary()
  @type proof() :: binary()

  @spec create() :: {:ok, identity :: t(), identity_id :: binary()} | {:error, reason :: any()}
  def create() do
    bytes = random()
    {:ok, "DATA_" <> bytes, "ID_" <> bytes}
  end

  @spec validate_identity_change_history(contact :: t()) ::
          {:ok, contact_id :: binary()} | {:error, reason :: any()}
  def validate_identity_change_history("DATA_" <> bytes) do
    {:ok, "ID_" <> bytes}
  end

  def validate_identity_change_history(other) do
    {:error, {:invalid_identity, other}}
  end

  @spec create_signature(identity :: t(), auth_hash :: binary()) ::
          {:ok, proof :: proof()} | {:error, reason :: any()}
  def create_signature(identity, auth_hash) do
    {:ok, identity <> auth_hash}
  end

  @spec verify_signature(
          identity :: t(),
          proof :: proof(),
          auth_hash :: binary()
        ) :: :ok | {:error, reason :: any()}
  def verify_signature(identity, proof, auth_hash) do
    {:ok, signature} = create_signature(identity, auth_hash)

    case proof do
      ^signature -> :ok
      other -> {:error, {:invalid_signature, other, proof}}
    end
  end

  def compare_identity_change_history(identity, identity) do
    {:ok, :equal}
  end

  def compare_identity_change_history(identity, known_identity) do
    {:error, {:history_update_not_supported, identity, known_identity}}
  end

  def random() do
    :rand.uniform(100_000) |> to_string() |> Base.encode32()
  end
end
