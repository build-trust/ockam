defmodule Ockam.Identity.Sidecar do
  @moduledoc """
  API facade for identity storage implemented in a sidecar node.
  Data structure contains a reference to identity stored
  in the sidecar node.
  """
  alias Ockam.API.Client, as: ApiClient
  alias Ockam.API.Response, as: ApiResponse

  alias Ockam.Identity.API.Request, as: IdentityRequest
  alias Ockam.Identity.API.Response, as: IdentityResponse

  @type t() :: binary()
  @type proof() :: binary()

  ## TODO: create proxy service to identity api in sidecar
  @api_route ["sidecar_identity_api"]

  @spec create() :: {:ok, identity :: t(), identity_id :: binary()} | {:error, reason :: any()}
  def create() do
    with {:ok, body} <- api_request(:post, api_path(:create), IdentityRequest.create()),
         {:ok, identity_response} <- IdentityResponse.create(body) do
      {:ok, Map.fetch!(identity_response, :identity), Map.fetch!(identity_response, :identity_id)}
    end
  end

  @spec validate_identity_change_history(contact :: t()) ::
          {:ok, contact_id :: binary()} | {:error, reason :: any()}
  def validate_identity_change_history(contact) do
    with {:ok, body} <-
           api_request(
             :post,
             api_path(:validate_identity_change_history),
             IdentityRequest.validate_identity_change_history(contact)
           ),
         {:ok, validate_response} <- IdentityResponse.validate_identity_change_history(body) do
      contact_id = Map.fetch!(validate_response, :identity_id)
      {:ok, contact_id}
    end
  end

  @spec create_signature(identity :: t(), auth_hash :: binary()) ::
          {:ok, proof :: proof()} | {:error, reason :: any()}
  def create_signature(identity, auth_hash) do
    with {:ok, body} <-
           api_request(
             :post,
             api_path(:create_signature),
             IdentityRequest.create_signature(identity, auth_hash)
           ),
         {:ok, create_signature_response} <- IdentityResponse.create_signature(body) do
      {:ok, Map.fetch!(create_signature_response, :proof)}
    end
  end

  @spec verify_signature(
          identity :: t(),
          proof :: proof(),
          auth_hash :: binary()
        ) :: :ok | {:error, reason :: any()}
  def verify_signature(identity, proof, auth_hash) do
    with {:ok, body} <-
           api_request(
             :post,
             api_path(:verify_signature),
             IdentityRequest.verify_signature(identity, proof, auth_hash)
           ),
         {:ok, verify_response} <- IdentityResponse.verify_signature(body) do
      case Map.fetch!(verify_response, :verified) do
        true -> :ok
        false -> {:error, :invalid_proof}
      end
    end
  end

  @spec compare_identity_change_history(current_identity :: t(), known_identity :: t) ::
          {:ok, atom()} | {:error, reason :: any()}
  def compare_identity_change_history(current_identity, known_identity) do
    with {:ok, body} <-
           api_request(
             :post,
             api_path(:compare_identity_change_history),
             IdentityRequest.compare_identity_change_history(current_identity, known_identity)
           ) do
      IdentityResponse.compare_identity_change_history(body)
    end
  end

  defp api_route() do
    @api_route
  end

  defp api_path(:create) do
    ""
  end

  defp api_path(:create_signature) do
    "actions/create_signature"
  end

  defp api_path(:validate_identity_change_history) do
    "actions/validate_identity_change_history"
  end

  defp api_path(:verify_signature) do
    "actions/verify_signature"
  end

  defp api_path(:compare_identity_change_history) do
    "actions/compare_identity_change_history"
  end

  defp api_request(method, path, request) do
    case ApiClient.sync_request(method, path, request, api_route()) do
      {:ok, %ApiResponse{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %ApiResponse{status: status}} ->
        {:error, {:api_error, status}}

      {:error, _} = error ->
        error
    end
  end
end
