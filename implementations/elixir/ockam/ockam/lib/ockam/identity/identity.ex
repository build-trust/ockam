defmodule Ockam.Identity do
  @moduledoc """
  API facade for identity implementations
  Using module name and opaque data to represent implementation-specific identities

  You can chose an implementation when creating an identity

  Default implementation is `Ockam.Identity.Sidecar`
  """

  @type t() :: {module :: atom, opaque :: binary()}
  @type proof() :: binary()

  def default_implementation() do
    Application.get_env(:ockam, :identity_module, Ockam.Identity.Stub)
  end

  @spec create(module :: atom()) ::
          {:ok, identity :: t(), identity_id :: binary()} | {:error, reason :: any()}
  def create(module \\ nil)

  def create(nil) do
    create(default_implementation())
  end

  def create(module) do
    with {:ok, data, id} <- module.create() do
      {:ok, {module, data}, id}
    end
  end

  def get(identity_name) do
    get(default_implementation(), identity_name)
  end

  def get(module, identity_name) do
    with {:ok, data, id} <- module.get(identity_name) do
      {:ok, {module, data}, id}
    end
  end

  def make_identity(identity) do
    make_identity(default_implementation(), identity)
  end

  def make_identity(module, {module, data}) do
    {:ok, {module, data}}
  end

  def make_identity(module, {other_module, _data}) do
    {:error, {:different_identity_implementations, module, other_module}}
  end

  def make_identity(module, data) when is_binary(data) do
    with {:ok, identity, _id} <- validate_contact_data({module, ""}, data) do
      {:ok, identity}
    end
  end

  def from_data(module, data) do
    validate_contact_data({module, ""}, data)
  end

  @spec validate_contact_data(my_identity :: t(), contact_data :: binary()) ::
          {:ok, identity :: t(), identity_id :: binary()}
  def validate_contact_data({my_module, _my_data}, contact_data) do
    with {:ok, contact_id} <- validate_identity_change_history({my_module, contact_data}) do
      {:ok, {my_module, contact_data}, contact_id}
    end
  end

  @spec get_data(t()) :: any()
  def get_data({_module, data}) do
    data
  end

  @spec validate_identity_change_history(contact :: t()) ::
          {:ok, contact_id :: binary()} | {:error, reason :: any()}
  def validate_identity_change_history({module, data}) do
    module.validate_identity_change_history(data)
  end

  @spec create_signature(identity :: t(), auth_hash :: binary()) ::
          {:ok, proof :: proof()} | {:error, reason :: any()}
  @spec create_signature(identity :: t(), auth_hash :: binary(), vault_name :: String.t() | nil) ::
          {:ok, proof :: proof()} | {:error, reason :: any()}
  def create_signature({module, data}, auth_hash, vault_name \\ nil) do
    module.create_signature(vault_name, data, auth_hash)
  end

  @spec verify_signature(
          identity :: t(),
          proof :: proof(),
          auth_hash :: binary()
        ) :: :ok | {:error, reason :: any()}
  def verify_signature({module, data}, proof, auth_hash) do
    module.verify_signature(data, proof, auth_hash)
  end

  @spec compare_identity_change_history(current_identity :: t(), known_identity :: t) ::
          {:ok, atom()} | {:error, reason :: any()}
  def compare_identity_change_history({module, current_data}, {module, known_data}) do
    module.compare_identity_change_history(current_data, known_data)
  end

  def compare_identity_change_history(current_identity, known_identity) do
    {:error, {:different_identity_implementations, current_identity, known_identity}}
  end
end
