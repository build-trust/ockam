defmodule Ockam.Services.TokenLeaseManager.CloudService.Influxdb do
  @moduledoc false
  @behaviour Ockam.Services.TokenLeaseManager.CloudService
  alias Ockam.Services.TokenLeaseManager.Lease
  require Logger

  @auth_url "/api/v2/authorizations"
  @http_ok 200
  @http_created 201
  @http_delete 204
  @http_not_found 404

  @impl true
  def init(options) do
    endpoint = options[:endpoint]
    token = options[:token]
    org_id = options[:org_id]
    {:ok, permissions} = Poison.decode(options[:permissions])

    {:ok,
     [
       endpoint: endpoint,
       token: token,
       auth_url: @auth_url,
       org_id: org_id,
       permissions: permissions
     ]}
  end

  @impl true
  def create(cloud_configuration, identity_id, ttl) do
    url = build_url(cloud_configuration)
    headers = build_headers(cloud_configuration)

    expires = DateTime.add(DateTime.utc_now(), ttl, :second)

    options = %{
      "permissions" => cloud_configuration[:permissions],
      "orgID" => cloud_configuration[:org_id],
      "description" => "ockam/#{identity_id}/#{DateTime.to_iso8601(expires)}"
    }

    {:ok, body} = Poison.encode(options)

    request(
      fn -> HTTPoison.post(url, body, headers) end,
      fn body -> decode_one(body) end,
      @http_created
    )
  end

  @impl true
  def revoke(cloud_configuration, token_id) do
    url = "#{build_url(cloud_configuration)}/#{token_id}"
    headers = build_headers(cloud_configuration)
    request(fn -> HTTPoison.delete(url, headers) end, nil, @http_delete)
  end

  @impl true
  def get_all(cloud_configuration) do
    url = build_url(cloud_configuration)
    headers = build_headers(cloud_configuration)
    request(fn -> HTTPoison.get(url, headers) end, fn body -> decode_all(body) end)
  end

  defp request(req, decode, right_status_code \\ @http_ok) do
    case req.() do
      {:ok, %HTTPoison.Response{status_code: ^right_status_code, body: body}} ->
        if decode != nil do
          decode.(body)
        else
          :ok
        end

      {:ok, %HTTPoison.Response{status_code: @http_not_found}} ->
        :not_found

      {:ok, %HTTPoison.Response{status_code: status_code, body: body}} ->
        case Poison.decode(body) do
          {:ok, %{"code" => code, "message" => message}} ->
            {:error, "#{status_code}: #{code} => #{message}"}

          {:error, error} ->
            {:error, error}
        end

      {:error, %HTTPoison.Error{reason: reason}} ->
        {:error, reason}
    end
  end

  defp decode_all(body) do
    case Poison.decode(body) do
      {:ok, %{"authorizations" => authorizations}} ->
        {:ok,
         Enum.flat_map(
           authorizations,
           fn auth ->
             case parse_lease(auth) do
               {:ok, lease} ->
                 [lease]

               {:error, error} ->
                 Logger.info("Discarding auth token #{auth["id"]} due to: #{inspect(error)}")
                 []
             end
           end
         )}

      {:error, err} ->
        {:error, err}
    end
  end

  defp parse_lease(auth) do
    ockam_metadata = auth["description"]

    with ["ockam", issued_for, expires] <- String.split(ockam_metadata, "/"),
         {:ok, _expire_date, _offset} <- DateTime.from_iso8601(expires) do
      {:ok,
       %Lease{
         id: auth["id"],
         issued_for: issued_for,
         expires: expires,
         issued: auth["createdAt"],
         value: auth["token"],
         status: auth["status"]
       }}
    else
      other ->
        {:error, {:unknown_metadata, ockam_metadata, other}}
    end
  end

  defp decode_one(body) do
    with {:ok, auth} <- Poison.decode(body) do
      parse_lease(auth)
    end
  end

  defp build_url(
         endpoint: endpoint,
         token: _token,
         auth_url: auth_url,
         org_id: _org_id,
         permissions: _permissions
       ) do
    "#{endpoint}#{auth_url}"
  end

  defp build_headers(
         endpoint: _endpoint,
         token: token,
         auth_url: _auth_url,
         org_id: _org,
         permissions: _permissions
       ) do
    [
      Authorization: "Token #{token}",
      "Content-Type": "application/json"
    ]
  end
end
