defmodule Ockam.TokenLeaseManager.CloudServiceInfluxdb do
  @behaviour Ockam.TokenLeaseManager.TokenCloudService

  alias Ockam.TokenLeaseManager.Lease

  @endpoint_autorizations "/api/v2/authorizations"
  @http_ok 200
  @http_created 201
  @http_delete 204

  def configuration() do
    host = Application.get_env(:ockam_hub, :influxdb)[:host]
    port = Application.get_env(:ockam_hub, :influxdb)[:port]
    token = Application.get_env(:ockam_hub, :influxdb)[:token]
    org = Application.get_env(:ockam_hub, :influxdb)[:org]
    [host: host, port: port, token: token, endpoint: @endpoint_autorizations, org: org]
  end

  def create(options, cloud_service_config \\ []) do
    config = get_service_configuration(cloud_service_config)
    url = build_url(config)
    headers = build_headers(config)
    {:ok, body} = Poison.encode(options)
    request(fn -> HTTPoison.post(url, body, headers) end, fn body -> decode_one(body) end, @http_created)
  end

  def revoke(token_id, cloud_service_config \\ []) do
    config = get_service_configuration(cloud_service_config)
    url = "#{build_url(config)}/#{token_id}"
    headers = build_headers(config)
    request(fn -> HTTPoison.delete(url, headers) end, nil, @http_delete)
  end

  def renew(token_id, cloud_service_config \\ []) do
    config = get_service_configuration(cloud_service_config)
    #TODO
  end

  def get(token_id, cloud_service_config \\ []) do
    config = get_service_configuration(cloud_service_config)
    url = "#{build_url(config)}/#{token_id}"
    headers = build_headers(config)
    request(fn -> HTTPoison.get(url, headers) end, fn body -> decode_one(body) end)
  end

  def get_all(cloud_service_config \\ []) do
    config = get_service_configuration(cloud_service_config)
    url = build_url(config)
    headers = build_headers(config)
    request(fn -> HTTPoison.get(url, headers) end, fn body -> decode_all(body) end)
  end

  defp request(req, decode \\ nil, right_status_code \\ @http_ok) do
    case req.() do
      {:ok, %HTTPoison.Response{status_code: ^right_status_code, body: body}} ->
        if decode != nil do
          decode.(body)
        else
          :ok
        end

      {:ok, %HTTPoison.Response{status_code: status_code, body: body}} ->
        case Poison.Parser.parse(body) do
          {:ok, %{"code" => code, "message" => message}} -> {:error, "#{status_code}: #{code} => #{message}"}
          {:error, error} -> {:error, error}
        end

      {:error, %HTTPoison.Error{reason: reason}} ->
        {:error, reason}
    end
  end

  defp decode_all(body) do
    case Poison.Parser.parse(body) do
      {:ok, %{"authorizations" => authorizations}} ->
        {:ok, Enum.map(
          authorizations,
          fn auth ->
            IO.inspect(auth)
            %Lease{
              id: auth["id"],
              issued: auth["createdAt"], #TODO convert to datetime
              value: auth["token"]
            }
          end
        )
        }
      {:error, err} -> {:error, err}
    end
  end

  defp decode_one(body) do
    case Poison.Parser.parse(body) do
      {:ok, auth} ->
        {:ok,
          %Lease{
            id: auth["id"],
            issued: auth["createdAt"], #TODO convert to datetime
            value: auth["token"]
          }
        }
      {:error, err} -> {:error, err}
    end
  end

  defp get_service_configuration(config \\ []) do
    case Enum.empty?(config) do
      true -> configuration()
      false -> config
    end
  end

  defp build_url([host: host, port: port, token: token, endpoint: endpoint, org: _org]) do
    "#{host}:#{port}#{endpoint}"
  end

  defp build_headers([host: _host, port: _port, token: token, endpoint: _endpoint, org: _org]) do
    [
      "Authorization": "Token #{token}",
      "Content-Type": "application/json"
    ]
  end

end
