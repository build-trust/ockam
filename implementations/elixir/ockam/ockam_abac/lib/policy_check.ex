defmodule Ockam.ABAC.PolicyCheck do
  @moduledoc """
  Policy Decision Point for Ockam.ABAC

  Provides functions to check if ABAC.Request matches policies.
  """

  alias Ockam.ABAC.Policy
  alias Ockam.ABAC.Request

  def with_check(%Request{} = request, policies_or_storage, fun)
      when is_list(policies_or_storage) or is_atom(policies_or_storage) do
    with :ok <- match_policies(request, policies_or_storage) do
      fun.()
    end
  end

  def match_policies(%Request{} = request, policies) when is_list(policies) do
    case Enum.any?(policies, fn policy -> Policy.match_policy?(policy, request) end) do
      true -> :ok
      false -> {:error, {:abac_policy_mismatch, policies}}
    end
  end

  def match_policies(%Request{action_id: action_id} = request, storage) when is_atom(storage) do
    case get_policy(storage, action_id) do
      {:ok, policy} ->
        case Policy.match_policy?(policy, request) do
          true -> :ok
          false -> {:error, {:abac_policy_mismatch, [policy]}}
        end

      {:error, _reason} ->
        {:error, {:abac_policy_mismatch, []}}
    end
  end

  def get_policy(storage, action_id) when is_atom(storage) do
    storage.get_policy(action_id)
  end
end
