defmodule Ockam.Router do
  alias __MODULE__.Protocol.Endpoint

  @opaque connection :: %{__struct__: module()}

  @doc """
  Connects to the given endpoint

  Certain endpoints may be treated specially, such as when Influx is hosted
  within the router itself; this allows the router to act in both its primary
  capacity, and as the service shim layer for such services.
  """
  @spec connect(Endpoint.t()) :: {:ok, connection} | {:error, term}
  def connect(dest)

  def connect(%Endpoint{value: %Endpoint.Local{data: name}}) do
    case Ockam.Registry.lookup(name) do
      nil ->
        {:error, :not_found}

      {pid, service} when is_pid(pid) and is_atom(service) ->
        service.connect(pid, self())
    end
  end

  def connect(%Endpoint{}), do: {:error, :unsupported}

  @doc "Send a message to the given connection"
  @spec send(connection, term) :: :ok | {:error, term}
  def send(%mod{} = connection, message) do
    mod.send(connection, message)
  end

  @doc "Execute a request to the given connection"
  @spec request(connection, term) :: {:ok, reply :: term} | {:error, term}
  def request(%mod{} = connection, req) do
    mod.request(connection, req)
  end
end
