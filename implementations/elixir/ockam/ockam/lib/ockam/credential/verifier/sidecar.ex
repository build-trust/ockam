defmodule Ockam.Credential.Verifier.Sidecar do
  @moduledoc """
  API wrapper to call credential verification using sidecar verifier service
  """
  ## TODO: sidecar workers should maybe be moved to another application like ockam_services
  alias Ockam.API.Client, as: ApiClient
  alias Ockam.API.Response, as: ApiResponse

  alias Ockam.Credential.VerifyRequest

  alias Ockam.Credential.AttributeSet

  @api_route ["ca_verifier_sidecar"]

  def api_route() do
    @api_route
  end

  @spec verify(
          credential :: binary(),
          subject_id :: String.t(),
          authorities :: %{String.t() => binary()}
        ) :: {:ok, attribute_set :: AttributeSet.t()} | {:error, reason :: any()}
  def verify(credential, subject_id, authorities)
      when is_binary(credential) and is_binary(subject_id) and is_map(authorities) do
    method = :post
    path = "verify"

    request =
      VerifyRequest.encode!(%VerifyRequest{
        credential: credential,
        subject_id: subject_id,
        authorities: authorities
      })

    case ApiClient.sync_request(method, path, request, api_route()) do
      {:ok, %ApiResponse{status: 200, body: body}} ->
        case AttributeSet.decode(body) do
          {:ok, %AttributeSet{} = attribute_set, ""} ->
            {:ok, attribute_set}

          {:error, reason} ->
            {:error, reason}
        end

      {:ok, %ApiResponse{status: status}} ->
        {:error, {:api_error, status}}

      {:error, _} = error ->
        error
    end
  end
end
