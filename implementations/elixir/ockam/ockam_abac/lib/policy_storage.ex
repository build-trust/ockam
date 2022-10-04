defmodule Ockam.ABAC.PolicyStorage do
  @moduledoc """
  Behaviour describing access to the policy storage.
  Used in PolicyCheck to retrieve policies based on ABAC.ActionId
  """

  alias Ockam.ABAC.ActionId
  alias Ockam.ABAC.Policy

  @callback list() :: {:ok, [Policy.t()]} | {:error, any()}
  @callback get_policy(ActionId.t()) :: {:ok, Policy.t()} | {:error, any()}
  @callback put_policy(Policy.t()) :: :ok | {:error, any()}
  @callback delete_policy(ActionId.t()) :: :ok | {:error, any()}

  def list() do
    with {:ok, storage} <- Ockam.ABAC.default_policy_storage() do
      storage.list()
    end
  end

  def get_policy(action_id) do
    with {:ok, storage} <- Ockam.ABAC.default_policy_storage() do
      storage.get_policy(action_id)
    end
  end

  def put_policy(policy) do
    with {:ok, storage} <- Ockam.ABAC.default_policy_storage() do
      storage.put_policy(policy)
    end
  end

  def delete_policy(action_id) do
    with {:ok, storage} <- Ockam.ABAC.default_policy_storage() do
      storage.delete_policy(action_id)
    end
  end
end
