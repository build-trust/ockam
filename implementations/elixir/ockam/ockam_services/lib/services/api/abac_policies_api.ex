defmodule Ockam.Services.API.ABAC.PoliciesApi do
  @moduledoc """
  API to set/delete ABAC policies
  """
  ## TODO: move to ockam_abac after moving Ockam.Services.API to ockam
  use Ockam.Services.API

  alias Ockam.API.Request

  alias Ockam.ABAC.ActionId
  alias Ockam.ABAC.Policy
  alias Ockam.ABAC.PolicyStorage

  alias Ockam.TypedCBOR

  @impl true
  def setup(_options, state) do
    {:ok, state}
  end

  defp encode_list_response(policies) do
    formatted_policies =
      Enum.map(policies, fn policy ->
        action_id = ActionId.format(policy.action_id)
        attribute_rule = Policy.to_rule_string(policy)
        {action_id, attribute_rule}
      end)
      |> Map.new()

    TypedCBOR.encode!({:map, :string, :string}, formatted_policies)
  end

  @impl true
  def handle_request(%Request{method: :get, path: ""}, state) do
    ## TODO: different access permissions for policies
    with {:ok, policies} <- PolicyStorage.list() do
      response = encode_list_response(policies)
      {:reply, :ok, response, state}
    end
  end

  def handle_request(%Request{method: :get, path: path}, state) do
    with_action_path(path, fn action_id ->
      with {:ok, policy} <- PolicyStorage.get_policy(action_id) do
        response = Policy.to_rule_string(policy)
        {:reply, :ok, response, state}
      end
    end)
  end

  def handle_request(%Request{method: :put, path: path, body: data}, state) do
    with_action_path(path, fn action_id ->
      case Policy.from_rule_string(data, action_id) do
        {:ok, policy} ->
          with :ok <- PolicyStorage.put_policy(policy) do
            {:reply, :ok, nil, state}
          end

        {:error, _decode_error} ->
          {:error, {:bad_request, :cannot_decode_policy}}
      end
    end)
  end

  def handle_request(%Request{method: :delete, path: path}, state) do
    with_action_path(path, fn action_id ->
      with :ok <- PolicyStorage.delete_policy(action_id) do
        {:reply, :ok, nil, state}
      end
    end)
  end

  def with_action_path(path, fun) do
    case ActionId.parse(path) do
      {:ok, action_id} ->
        fun.(action_id)

      {:error, :invalid_path} ->
        {:error, {:bad_request, :invalid_path}}
    end
  end
end
