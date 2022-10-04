defmodule Ockam.ABAC.Policy do
  @moduledoc """
  Policy data structore for Ockam ABAC
  """

  alias Ockam.ABAC.ActionId
  alias Ockam.ABAC.AttributeRule
  alias Ockam.ABAC.Request

  @keys [:action_id, :attribute_rule]
  @enforce_keys @keys
  defstruct @keys

  @type t() :: %__MODULE__{
          action_id: ActionId.t(),
          attribute_rule: AttributeRule.t()
        }

  def match_policy?(%__MODULE__{} = policy, %Request{} = request) do
    case ActionId.match_action_id?(policy.action_id, request.action_id) do
      true ->
        AttributeRule.match_rule?(policy.attribute_rule, request)

      false ->
        false
    end
  end

  def from_rule_string(rules_str, action_id) do
    with {:ok, attribute_rule} <- AttributeRule.parse(rules_str) do
      {:ok, %__MODULE__{action_id: action_id, attribute_rule: attribute_rule}}
    end
  end

  def to_rule_string(%__MODULE__{attribute_rule: attribute_rule}) do
    AttributeRule.format(attribute_rule)
  end
end
