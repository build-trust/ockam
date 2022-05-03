defmodule Ockam.Services.TokenLeaseManager.StorageService.Postgres do
  @moduledoc false
  use Ockam.Services.TokenLeaseManager.StorageService

  alias Ockam.Services.TokenLeaseManager.Lease

  @opts_keys [:hostname, :username, :password, :database, :token_cloud_service]
  @table "leases"

  @impl true
  def handle_init({token_cloud_service, token_cloud_service_address, options}) do
    opts =
      options
      |> Keyword.put(:token_cloud_service, token_cloud_service)
      |> Keyword.put(:token_cloud_service_address, token_cloud_service_address)

    if check_options(opts) do
      case create_storage_conf(opts) do
        {:ok, conf} ->
          conf
          |> Map.get(:conn)
          |> create_table()

          {:ok, conf}

        error ->
          error
      end
    else
      {:error, "wrong postgres options"}
    end
  end

  @impl true
  def handle_save(
        %{
          conn: conn,
          token_cloud_service: token_cloud_service,
          token_cloud_service_address: token_cloud_service_address
        },
        lease
      ) do
    query =
      "INSERT INTO #{@table} (ID, CLOUD_SERVICE, CLOUD_SERVICE_ADDRESS, LEASE) VALUES ($1 , $2 , $3, $4);"

    with {:ok, encoded_lease} <- Poison.encode(lease),
         {:ok, _result} <-
           Postgrex.query(conn, query, [
             lease.id,
             token_cloud_service,
             token_cloud_service_address,
             encoded_lease
           ]) do
      :ok
    else
      error ->
        error
    end
  end

  @impl true
  def handle_get(
        %{
          conn: conn,
          token_cloud_service: token_cloud_service,
          token_cloud_service_address: token_cloud_service_address
        },
        lease_id
      ) do
    query =
      "SELECT * FROM #{@table} WHERE ID=$1 AND CLOUD_SERVICE=$2 AND CLOUD_SERVICE_ADDRESS=$3 LIMIT 1;"

    case Postgrex.query(conn, query, [lease_id, token_cloud_service, token_cloud_service_address]) do
      {:ok,
       %Postgrex.Result{
         rows: [[^lease_id, ^token_cloud_service, ^token_cloud_service_address, encoded_lease]]
       }} ->
        Poison.decode(encoded_lease, as: %Lease{})

      {:ok, _other} ->
        {:ok, nil}

      error ->
        error
    end
  end

  @impl true
  def handle_remove(
        %{
          conn: conn,
          token_cloud_service: token_cloud_service,
          token_cloud_service_address: token_cloud_service_address
        },
        lease_id
      ) do
    query = "DELETE FROM #{@table} WHERE ID=$1 AND CLOUD_SERVICE=$2 AND CLOUD_SERVICE_ADDRESS=$3;"

    case Postgrex.query(conn, query, [lease_id, token_cloud_service, token_cloud_service_address]) do
      {:ok, _other} -> :ok
      error -> error
    end
  end

  @impl true
  def handle_get_all(%{
        conn: conn,
        token_cloud_service: token_cloud_service,
        token_cloud_service_address: token_cloud_service_address
      }) do
    query = "SELECT * FROM #{@table} WHERE CLOUD_SERVICE=$1 AND CLOUD_SERVICE_ADDRESS=$2;"

    case Postgrex.query(conn, query, [token_cloud_service, token_cloud_service_address]) do
      {:ok, %Postgrex.Result{rows: leases}} ->
        {:ok,
         for [_id, ^token_cloud_service, ^token_cloud_service_address, encoded_lease] <- leases do
           {:ok, lease} = Poison.decode(encoded_lease, as: %Lease{})
           lease
         end}

      error ->
        error
    end
  end

  defp check_options(opts) do
    Enum.reduce_while(@opts_keys, true, fn key, current ->
      if current, do: {:cont, Keyword.has_key?(opts, key)}, else: {:halt, false}
    end)
  end

  defp create_storage_conf(opts) do
    {token_cloud_service, postres_opts} = Keyword.pop(opts, :token_cloud_service)

    {token_cloud_service_address, postres_opts} =
      Keyword.pop(postres_opts, :token_cloud_service_address)

    case Postgrex.start_link(postres_opts) do
      {:ok, pid} ->
        {:ok,
         %{
           conn: pid,
           token_cloud_service: token_cloud_service,
           token_cloud_service_address: token_cloud_service_address
         }}

      error ->
        error
    end
  end

  defp create_table(conn) do
    query = "CREATE TABLE #{@table} (
        ID VARCHAR(200) PRIMARY KEY,
        CLOUD_SERVICE VARCHAR(200),
        CLOUD_SERVICE_ADDRESS VARCHAR(200),
        LEASE JSON
      );"

    case Postgrex.query(conn, query, []) do
      {:ok, result} ->
        Logger.debug("table #{@table} was created in Postgres Storage System: #{inspect(result)}")

      {:error, error} ->
        Logger.debug(
          "table #{@table} was not created in Postgres Storage System: #{inspect(error)}"
        )
    end
  end
end
