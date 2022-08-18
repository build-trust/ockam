defmodule Ockam.Services.API.CredentialExchange do
  @moduledoc """
  API to accept credentials.

  Verifies credentials using provided verifier_module.
  Saves attributes in AttributeStorageETS table per identity.

  Options:

  - authorities :: [identity :: binary] - list of supported CA public keys
  - verifier_module - module to call verify/3 with, see Ockam.Credential.Verifier.Sidecar
  """
  use Ockam.Services.API

  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage

  alias Ockam.Identity

  @default_verifier Ockam.Credential.Verifier.Sidecar

  def set_authorities(worker, identities_data) when is_list(identities_data) do
    Ockam.Worker.call(worker, {:set_authorities, identities_data})
  end

  @impl true
  def setup(options, state) do
    ## TODO: API to update authorities
    authorities = Keyword.fetch!(options, :authorities) |> prepare_authorities()
    verifier_module = Keyword.get(options, :verifier_module, @default_verifier)

    :ok = AttributeStorage.init()

    {:ok, Map.merge(state, %{authorities: authorities, verifier_module: verifier_module})}
  end

  @impl true
  def handle_request(%Request{method: :post, path: "actions/present"} = request, state) do
    case request do
      %{body: credential, local_metadata: %{identity_id: subject_id}} ->
        authorities = Map.fetch!(state, :authorities)
        verifier_module = Map.fetch!(state, :verifier_module)

        with {:ok, attribute_set} <- verifier_module.verify(credential, subject_id, authorities),
             :ok <- AttributeStorage.put_attribute_set(subject_id, attribute_set) do
          {:reply, :ok, nil, state}
        end

      _other ->
        {:error, {:bad_request, "secure channel required"}}
    end
  end

  def handle_request(%Request{method: :post}, _state) do
    {:error, :not_found}
  end

  def handle_request(%Request{}, _state) do
    {:error, :method_not_allowed}
  end

  @impl true
  def handle_call({:set_authorities, identities_data}, _from, state) do
    new_authorities = prepare_authorities(identities_data)
    {:reply, :ok, Map.put(state, :authorities, new_authorities)}
  end

  defp prepare_authorities(authorities_config) when is_map(authorities_config) do
    authorities_config
  end

  defp prepare_authorities(authorities_config) when is_list(authorities_config) do
    Enum.map(authorities_config, fn identity_data ->
      with {:ok, identity} <- Identity.make_identity(identity_data),
           {:ok, identity_id} <- Identity.validate_identity_change_history(identity) do
        {identity_id, identity}
      end
    end)
    |> Map.new()
  end
end
