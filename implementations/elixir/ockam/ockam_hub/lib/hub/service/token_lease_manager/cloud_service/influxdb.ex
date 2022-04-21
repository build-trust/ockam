defmodule Ockam.TokenLeaseManager.CloudService.Influxdb do
  @moduledoc false
  use Ockam.TokenLeaseManager.CloudService

  alias Ockam.TokenLeaseManager.Lease

  @auth_url "/api/v2/authorizations"
  @default_permissions [%{"action" => "read", "resource" => %{"type" => "authorizations"}}]
  @http_ok 200
  @http_created 201
  @http_delete 204
  @http_not_found 404

  @impl true
  def handle_init(options) do
    endpoint = options[:endpoint]
    token = options[:token]
    org = options[:org]
    {:ok, [endpoint: endpoint, token: token, auth_url: @auth_url, org: org]}
  end

  @impl true
  def handle_create(cloud_configuration, options) do
    url = build_url(cloud_configuration)
    headers = build_headers(cloud_configuration)

    {:ok, body} =
      options
      |> fill_options(cloud_configuration[:org])
      |> Poison.encode()

    request(
      fn -> HTTPoison.post(url, body, headers) end,
      fn body -> decode_one(body) end,
      @http_created
    )
  end

  @impl true
  def handle_revoke(cloud_configuration, token_id) do
    url = "#{build_url(cloud_configuration)}/#{token_id}"
    headers = build_headers(cloud_configuration)
    request(fn -> HTTPoison.delete(url, headers) end, nil, @http_delete)
  end

  @impl true
  def handle_renew(_cloud_configuration, _token_id) do
    # TODO
    :ok
  end

  @impl true
  def handle_get(cloud_configuration, token_id) do
    url = "#{build_url(cloud_configuration)}/#{token_id}"
    headers = build_headers(cloud_configuration)
    request(fn -> HTTPoison.get(url, headers) end, fn body -> decode_one(body) end)
  end

  @impl true
  def handle_get_all(cloud_configuration) do
    url = build_url(cloud_configuration)
    headers = build_headers(cloud_configuration)
    request(fn -> HTTPoison.get(url, headers) end, fn body -> decode_all(body) end)
  end

  @impl true
  def handle_get_address(cloud_configuration) do
    {:ok, "#{cloud_configuration[:endpoint]}"}
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
         Enum.map(
           authorizations,
           fn auth ->
             %Lease{
               id: auth["id"],
               # TODO convert to datetime
               issued: auth["createdAt"],
               value: auth["token"]
             }
           end
         )}

      {:error, err} ->
        {:error, err}
    end
  end

  defp decode_one(body) do
    case Poison.decode(body) do
      {:ok, auth} ->
        {:ok,
         %Lease{
           id: auth["id"],
           issued: auth["createdAt"],
           value: auth["token"]
         }}

      {:error, err} ->
        {:error, err}
    end
  end

  defp build_url(endpoint: endpoint, token: _token, auth_url: auth_url, org: _org) do
    "#{endpoint}#{auth_url}"
  end

  defp build_headers(endpoint: _endpoint, token: token, auth_url: _auth_url, org: _org) do
    [
      Authorization: "Token #{token}",
      "Content-Type": "application/json"
    ]
  end

  defp fill_options(%{"orgID" => _o, "permissions" => _p} = creation_options, _org),
    do: creation_options

  defp fill_options(%{"orgID" => _o} = creation_options, _org) do
    Map.put(creation_options, "permissions", @default_permissions)
  end

  defp fill_options(%{"permissions" => _p} = creation_options, org) do
    Map.put(creation_options, "orgID", org)
  end

  defp fill_options(creation_options, org) do
    creation_options
    |> Map.put("orgID", org)
    |> Map.put("permissions", @default_permissions)
  end
end
