defmodule Ockam.Services.Influx.Connection do
  use GenServer

  alias Ockam.Services.Influx.Message
  alias Ockam.Services.Influx.HTTP
  alias Ockam.Router.Protocol.Decoder

  def child_spec(args) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, [args]},
      restart: :temporary,
      shutdown: 1_000,
      type: :worker
    }
  end

  defmodule State do
    defstruct [:pid, :monitor, :host, :port, :db]
  end

  defstruct [:pid, :connected]

  @doc false
  def new(connection_pid, connecting_pid)
      when is_pid(connection_pid) and is_pid(connecting_pid) do
    %__MODULE__{pid: connection_pid, connected: connecting_pid}
  end

  @doc "Unpack and send the encoded message"
  def send(%__MODULE__{} = conn, encoded) do
    with {:ok, %Message{value: decoded}, _} <- Decoder.decode(%Message{}, encoded, %{}) do
      do_send(conn, decoded)
    end
  end

  defp do_send(conn, %Message.Write{measurement: m, tags: tags, fields: fields}) do
    write(conn, m, tags, fields)
  end

  defp do_send(_conn, _msg) do
    {:error, :invalid_send_type}
  end

  @doc "Unpack and execute the encoded request"
  def request(%__MODULE__{} = conn, encoded) do
    with {:ok, %Message{value: decoded}, _} <- Decoder.decode(%Message{}, encoded, %{}) do
      do_request(conn, decoded)
    end
  end

  defp do_request(conn, %Message.Query{text: text}) do
    query(conn, text)
  end

  defp do_request(_conn, _), do: {:error, :invalid_request_type}

  @doc "Disconnect this connection"
  def disconnect(%__MODULE__{pid: pid}) do
    GenServer.call(pid, :disconnect)
  catch
    :exit, {:noproc, _} ->
      {:error, :connection_closed}
  end

  @doc "Write data to a measurement in Influx"
  def write(%__MODULE__{pid: pid}, measurement, tags, fields) do
    GenServer.call(pid, {:write, measurement, tags, fields})
  catch
    :exit, {:noproc, _} ->
      {:error, :connection_closed}
  end

  @doc "Execute a query against the Influx database"
  def query(%__MODULE__{pid: pid}, text) when is_binary(text) do
    GenServer.call(pid, {:query, text})
  catch
    :exit, {:noproc, _} ->
      {:error, :connection_closed}
  end

  # GenServer impl

  def start_link(opts, pid), do: GenServer.start_link(__MODULE__, [opts, pid])

  def init([opts, pid]) when is_pid(pid) do
    ref = Process.monitor(pid)
    host = get_in(opts, [:http, :host]) || "localhost"
    port = get_in(opts, [:http, :port]) || 8086
    db = Keyword.fetch!(opts, :database)
    {:ok, %State{pid: pid, monitor: ref, host: host, port: port, db: db}}
  end

  def handle_call({:write, measurement, tags, fields}, _from, %State{} = state) do
    with {:ok, client} <- HTTP.connect(state.host, state.port),
         {:ok, data} <- encode_write(measurement, tags, fields),
         {:ok, client, %HTTP.Response{} = resp} <-
           HTTP.post(client, "/write", %{"db" => state.db, "precision" => "s"}, data),
         :ok <- HTTP.close(client) do
      case resp.status do
        n when n >= 200 and n < 300 ->
          {:reply, :ok, state}

        _error_status ->
          {:reply, {:error, resp.body}, state}
      end
    else
      {:error, _reason} = err ->
        {:reply, err, state}

      {:error, client, reason} ->
        HTTP.close(client)
        {:reply, {:error, reason}, state}
    end
  end

  def handle_call({:query, text}, _from, %State{} = state) do
    with {:ok, client} <- HTTP.connect(state.host, state.port),
         {:ok, client, %HTTP.Response{} = resp} <-
           HTTP.get(client, "/query", %{"db" => state.db, "q" => text}),
         :ok <- HTTP.close(client) do
      case resp.status do
        200 ->
          {:reply, {:ok, resp.body}, state}

        _error_status ->
          {:reply, {:error, resp.body}, state}
      end
    else
      {:error, _reason} = err ->
        {:reply, err, state}

      {:error, client, reason} ->
        HTTP.close(client)
        {:reply, {:error, reason}, state}
    end
  end

  def handle_call(:disconnect, from, %State{monitor: ref}) do
    Process.demonitor(ref)
    GenServer.reply(from, :ok)
    {:stop, :shutdown}
  end

  def handle_info({:DOWN, ref, :process, pid, reason}, %State{pid: pid, monitor: ref}) do
    {:stop, reason}
  end

  # measurement,tag_key=tag_value,... field_key=field_value
  defp encode_write(measurement, [], fields) do
    fields_encoded =
      fields
      |> Enum.map(fn {key, value} -> "#{escape(to_string(key))}=#{encode_value(value)}" end)
      |> Enum.join(",")

    {:ok, "#{measurement} #{fields_encoded}"}
  end

  defp encode_write(measurement, tags, fields) do
    tags_encoded =
      tags
      |> Enum.map(fn {key, value} -> "#{encode_value(key)}=#{encode_value(value)}" end)
      |> Enum.join(",")

    fields_encoded =
      fields
      |> Enum.map(fn {key, value} -> "#{encode_value(key)}=#{encode_value(value)}" end)
      |> Enum.join(",")

    {:ok, "#{measurement},#{tags_encoded} #{fields_encoded}"}
  end

  defp escape(value) do
    Regex.replace(~r/([,\s\\"])/, value, fn _, x -> "\\#{x}" end)
  end

  defp encode_value(s) when is_binary(s) do
    if Regex.match?(~r/[,\s\\"]/, s) do
      "\"#{escape(s)}\""
    else
      s
    end
  end

  defp encode_value(a) when is_atom(a) do
    encode_value(to_string(a))
  end

  defp encode_value(b) when is_boolean(b) do
    to_string(b)
  end

  defp encode_value(n) when is_number(n) do
    to_string(n)
  end
end
