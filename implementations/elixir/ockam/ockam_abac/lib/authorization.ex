defmodule Ockam.ABAC.Authorization do
  @moduledoc """
  Message flow authorization functions for Ockam.ABAC

  Provides a message authorization function to check policies based on message and worker state.
  """

  alias Ockam.ABAC.ActionId
  alias Ockam.ABAC.Request
  alias Ockam.Credential.AttributeStorageETS

  alias Ockam.Message

  require Logger

  @doc """
  Authorization function using ABAC.
  This function acts as a Policy Enforcement Point,
  it creates an ABAC.Request and sends it to ABAC.PolicyCheck

  `policies_or_storage` - either a list of Ockam.ABAC.Policy
  or a module implementing Ockam.ABAC.PolicyStorage
  By default takes storage implementation
  from :ockam_abac, :policy_storage app variable
  """
  def with_policy_check(message, state) do
    case Ockam.ABAC.default_policy_storage() do
      {:ok, policy_storage} ->
        with_policy_check(message, state, policy_storage)

      {:error, reason} ->
        Logger.warning("Policy storage error: #{inspect(reason)}")
        {:error, :abac_policy_missing}
    end
  end

  def with_policy_check(message, state, policies_or_storage) do
    request = generate_request(message, state)
    Ockam.ABAC.PolicyCheck.with_check(request, policies_or_storage, fn -> :ok end)
  end

  def authorization() do
    [{__MODULE__, :with_policy_check, [:message, :state]}]
  end

  def generate_request(message, state) do
    ## Using address as a resource ID,
    ## maybe we should use some other property in the worker state
    resource_id = Map.fetch!(state, :address)

    action_id = "handle_message"

    subject_attributes = subject_attributes(message)

    resource_attributes = resource_attributes(state)

    action_attributes = action_attributes(message)

    %Request{
      action_id: %ActionId{resource: resource_id, action: action_id},
      resource_attributes: resource_attributes,
      action_attributes: action_attributes,
      subject_attributes: subject_attributes
    }
  end

  def action_attributes(message) do
    to_attrs =
      case Message.onward_route(message) do
        [to | _rest] -> %{"to" => to}
        [] -> %{}
      end

    from_attrs =
      case Message.return_route(message) do
        [from | _rest] -> %{"from" => from}
        [] -> %{}
      end

    local_metadata = Message.local_metadata(message)

    local_metadata
    |> Map.drop([:identity_id, :identity])
    |> Map.merge(to_attrs)
    |> Map.merge(from_attrs)
    |> normalize_attributes()
  end

  def subject_attributes(message) do
    case Message.local_metadata(message) do
      %{identity_id: identity_id} ->
        ## TODO: do we want to make this optional?
        AttributeStorageETS.get_attributes(identity_id)
        |> Map.merge(%{"identity_id" => identity_id})
        |> normalize_attributes()

      _other ->
        %{}
    end
  end

  def resource_attributes(state) do
    Map.get(state, :attributes, %{})
    |> Map.merge(%{
      "address" => Map.fetch!(state, :address),
      "all_addresses" => Map.fetch!(state, :all_addresses)
    })
    |> normalize_attributes()
  end

  defp normalize_attributes(attributes) do
    Enum.flat_map(
      attributes,
      fn {key, val} ->
        case val do
          binary when is_binary(binary) -> [{to_string(key), binary}]
          atom when is_atom(atom) -> [{to_string(key), to_string(atom)}]
          [binary | _rest] = list when is_binary(binary) -> [{to_string(key), list}]
          ## TODO: handle other attribute types
          _other -> []
        end
      end
    )
    |> Map.new()
  end
end
