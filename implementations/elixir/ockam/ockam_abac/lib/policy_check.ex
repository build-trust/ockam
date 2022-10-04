defmodule Ockam.ABAC.PolicyCheck do
  @moduledoc """
  Policy Decision Point for Ockam.ABAC

  Provides functions to check if ABAC.Request matches policies.
  """

  alias Ockam.ABAC.Policy
  alias Ockam.ABAC.Request

  def with_check(%Request{} = request, policies_or_storage, fun)
      when is_list(policies_or_storage) or is_atom(policies_or_storage) do
    case is_authorized?(request, policies_or_storage) do
      true ->
        fun.()

      false ->
        ## TODO: expand error reason
        {:error, :abac_policy_mismatch}
    end
  end

  def is_authorized?(%Request{} = request, policies) when is_list(policies) do
    Enum.any?(policies, fn policy -> Policy.match_policy?(policy, request) end)
  end

  def is_authorized?(%Request{action_id: action_id} = request, storage) when is_atom(storage) do
    case get_policy(storage, action_id) do
      {:ok, policy} ->
        Policy.match_policy?(policy, request)

      {:error, _reason} ->
        false
    end
  end

  def get_policy(storage, action_id) when is_atom(storage) do
    storage.get_policy(action_id)
  end
end
