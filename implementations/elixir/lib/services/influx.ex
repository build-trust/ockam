defmodule Ockam.Services.Influx do
  use Supervisor

  alias __MODULE__.Connection

  def start_link([meta, opts]) when is_list(meta) and is_list(opts) do
    name = Keyword.get(meta, :name, __MODULE__)

    Supervisor.start_link(__MODULE__, [name, opts])
  end

  @impl true
  def init([name, opts]) do
    children = [
      __MODULE__.Fluxter.child_spec(),
      {__MODULE__.ConnectionSupervisor, [name, opts]}
    ]

    Supervisor.init(children, strategy: :rest_for_one)
  end

  @doc """
  Connect the given process to Influx as a psuedo-client

  All client operations are performed via the resulting connection, which
  multiplexes operations over the connection pool started by the service.

  Connections are automatically terminated if the originating process dies
  """
  @spec connect(pid | atom, pid) :: {:ok, Connection.t()} | {:error, term}
  defdelegate connect(sup, connecting_pid), to: __MODULE__.ConnectionSupervisor

  @doc "Disconnects an active connection"
  @spec disconnect(Connection.t()) :: :ok
  defdelegate disconnect(conn), to: __MODULE__.Connection

  @doc """
  Writes the given measurement with the provided tags and fields.

  Tags may be an empty list.
  """
  @spec write(Connection.t(), binary(), Keyword.t(), Keyword.t()) :: :ok
  defdelegate write(conn, measurement, tags, fields), to: __MODULE__.Connection

  @doc """
  Executes the given query (InfluxQL)
  """
  @spec query(Connection.t(), binary()) :: {:ok, binary()} | {:error, term}
  defdelegate query(conn, query_text), to: __MODULE__.Connection
end
