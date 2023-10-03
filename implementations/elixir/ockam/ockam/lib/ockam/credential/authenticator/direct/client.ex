defmodule Ockam.Credential.Authenticator.Direct.Client.AddMemberRequest do
  @moduledoc """
  Request data structure for Credential Authority authenticator API add_member
  """
  use TypedStruct

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)

    field(:identity_id, Ockam.Identity.Identifier.t(),
      minicbor: [key: 1, schema: Ockam.Identity.Identifier]
    )

    field(:attributes, %{String.t() => String.t()}, minicbor: [key: 2])
  end
end

defmodule Ockam.Credential.Authenticator.Direct.Client.AttributesEntry do
  @moduledoc """
  Data structure representing attribute entry for Credential Authority authenticator API list_members
  """
  use TypedStruct

  alias Ockam.Identity.Identifier

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    ## Rust encodes attribute values as a list of bytes
    field(:attributes, %{{:list, :integer} => {:list, :integer}}, minicbor: [key: 1])
    field(:added_at, integer(), minicbor: [key: 2])
    field(:expires, integer() | nil, minicbor: [key: 3])

    field(:attested_by, Identifier.t() | nil, minicbor: [key: 4, schema: Identifier])
  end
end

defmodule Ockam.Credential.Authenticator.Direct.Client.ListMembersResponse do
  @moduledoc """
  Response data structure for Credential Authority authenticator API list_members

  This API returns a map of identity identifiers to attribute entries
  """

  alias Ockam.Credential.Authenticator.Direct.Client.AttributesEntry
  alias Ockam.Identity.Identifier

  @schema {:map, :binary, AttributesEntry.minicbor_schema()}

  def decode_strict(data) do
    with {:ok, decoded} <- Ockam.TypedCBOR.decode_strict(@schema, data) do
      ## Convert attribute values to binaries

      {:ok,
       Map.new(decoded, fn {id, entry} ->
         attributes =
           Map.get(entry, :attributes, %{})
           |> Map.new(fn {key, val} ->
             {:erlang.list_to_binary(key), :erlang.list_to_binary(val)}
           end)

         {%Identifier{id: id}, Map.put(entry, :attributes, attributes)}
       end)}
    end
  end
end

defmodule Ockam.Credential.Authenticator.Direct.Client do
  @moduledoc """
  API client for Credential Authority authenticator service

  Allows to add new members to credential authority
  """

  alias Ockam.API.Client, as: ApiClient
  alias Ockam.API.Response, as: ApiResponse

  alias Ockam.Credential.Authenticator.Direct.Client.AddMemberRequest
  alias Ockam.Credential.Authenticator.Direct.Client.AttributesEntry
  alias Ockam.Credential.Authenticator.Direct.Client.ListMembersResponse

  @spec add_member(
          Ockam.Identity.Identifier.t(),
          %{String.t() => String.t()},
          Ockam.Address.route()
        ) ::
          :ok | {:error, any()}
  def add_member(identity_id, attributes, api_route) do
    request =
      AddMemberRequest.encode!(%AddMemberRequest{
        identity_id: identity_id,
        attributes: attributes
      })

    case ApiClient.sync_request(:post, "/", request, api_route) do
      {:ok, %ApiResponse{status: 200}} ->
        :ok

      {:ok, %ApiResponse{status: status, body: body}} ->
        {:error, {:api_error, status, body}}

      {:error, _} = error ->
        error
    end
  end

  @spec list_member_ids(Ockam.Address.route()) :: {:ok, [String.t()]} | {:error, any()}
  def list_member_ids(api_route) do
    case ApiClient.sync_request(:get, "/member_ids", "", api_route) do
      {:ok, %ApiResponse{status: 200, body: response}} ->
        Ockam.TypedCBOR.decode_strict({:list, :binary}, response)

      {:ok, %ApiResponse{status: status, body: body}} ->
        {:error, {:api_error, status, body}}

      {:error, _} = error ->
        error
    end
  end

  @spec list_members(Ockam.Address.route()) ::
          {:ok, %{Ockam.Identity.Identifier.t() => AttributesEntry.t()}} | {:error, any()}
  def list_members(api_route) do
    with {:ok, %ApiResponse{status: 200, body: response}} <-
           ApiClient.sync_request(:get, "/", "", api_route),
         {:ok, members} <- ListMembersResponse.decode_strict(response) do
      {:ok, members}
    else
      {:ok, %ApiResponse{status: status, body: body}} ->
        {:error, {:api_error, status, body}}

      {:error, _} = error ->
        error
    end
  end

  @spec delete_member(String.t(), Ockam.Address.route()) :: :ok | {:error, any()}
  def delete_member(identity_id, api_route) do
    case ApiClient.sync_request(:delete, "/#{identity_id}", "", api_route) do
      {:ok, %ApiResponse{status: 200}} ->
        :ok

      {:ok, %ApiResponse{status: status, body: body}} ->
        {:error, {:api_error, status, body}}

      {:error, _} = error ->
        error
    end
  end
end
