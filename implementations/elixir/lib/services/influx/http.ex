defmodule Ockam.Services.Influx.HTTP do
  defstruct [:conn, :host, :port]

  defmodule Response do
    defstruct [:status, :headers, :body]

    def new() do
      %__MODULE__{status: nil, headers: [], body: ""}
    end
  end

  def connect(host, port, opts \\ [])

  def connect(host, port, _opts) do
    with {:ok, conn} <- Mint.HTTP1.connect(:http, host, port, mode: :passive) do
      {:ok, %__MODULE__{conn: conn, host: host, port: port}}
    end
  end

  def get(conn, path, query \\ nil)

  def get(%__MODULE__{conn: conn} = state, path, nil) do
    with {:ok, conn, _req} <- Mint.HTTP1.request(conn, "GET", path, _headers = [], _body = nil),
         {:ok, conn, resp} <- recv(conn) do
      {:ok, %__MODULE__{state | conn: conn}, resp}
    else
      {:error, conn, reason} ->
        {:error, %__MODULE__{state | conn: conn}, reason}

      {:error, conn, reason, _resp} ->
        {:error, %__MODULE__{state | conn: conn}, reason}
    end
  end

  def get(%__MODULE__{} = state, path, query) when is_map(query) do
    get(state, path <> "?" <> URI.encode_query(query), nil)
  end

  def post(%__MODULE__{conn: conn} = state, path, query, body) do
    path = path <> "?" <> URI.encode_query(query)

    with {:ok, conn, _req} <- Mint.HTTP1.request(conn, "POST", path, _headers = [], body),
         {:ok, conn, resp} <- recv(conn) do
      {:ok, %__MODULE__{state | conn: conn}, resp}
    else
      {:error, conn, reason} ->
        {:error, %__MODULE__{state | conn: conn}, reason}

      {:error, conn, reason, _resp} ->
        {:error, %__MODULE__{state | conn: conn}, reason}
    end
  end

  def close(%__MODULE__{conn: nil}), do: :ok

  def close(%__MODULE__{conn: http}) do
    Mint.HTTP1.close(http)
    :ok
  end

  defp recv(conn) do
    do_recv(conn, Response.new())
  end

  defp do_recv(conn, acc) do
    with {:ok, conn, responses} <- Mint.HTTP1.recv(conn, 0, :infinity),
         {:more, conn, new_acc} <- handle_responses(responses, conn, acc) do
      do_recv(conn, new_acc)
    end
  end

  defp handle_responses([], conn, acc), do: {:more, conn, acc}
  defp handle_responses([{:done, _}], conn, acc), do: {:ok, conn, acc}

  defp handle_responses([{:status, _, status} | rest], conn, acc) do
    handle_responses(rest, conn, %Response{acc | status: status})
  end

  defp handle_responses([{:headers, _, headers} | rest], conn, acc) do
    handle_responses(rest, conn, %Response{acc | headers: acc.headers ++ headers})
  end

  defp handle_responses([{:data, _, chunk} | rest], conn, acc) do
    handle_responses(rest, conn, %Response{acc | body: acc.body <> chunk})
  end
end
