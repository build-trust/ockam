defmodule Ockam.Transport.Socket do
  @moduledoc """
  An implementation of the `Ockam.Transport` behavior for TCP sockets
  """
  alias Ockam.Transport
  alias Ockam.Transport.Address

  @behaviour Ockam.Transport

  @roles [:client, :server]

  defstruct role: :client,
            socket: nil,
            buffer: "",
            address: nil

  @doc "Creates a new socket configuration"
  def new(role, address)

  def new(role, address), do: new(role, nil, address)

  def new(role, socket, %Address{} = address) when role in @roles do
    %__MODULE__{
      role: role,
      socket: socket,
      buffer: "",
      address: address
    }
  end

  @doc "Initializes the socket configuration"
  def init(opts) do
    # Ensure we raise an error if role was not provided
    role = Keyword.fetch!(opts, :role)
    address = Keyword.fetch!(opts, :address)
    new(role, address)
  end

  @doc "Opens the socket using the provided configuration"
  def open(%__MODULE__{role: role, socket: nil} = state) do
    with {:ok, socket} <- :socket.open(:inet, :stream, :tcp),
         :ok <- :socket.setopt(socket, :socket, :keepalive, true),
         :ok <- :socket.setopt(socket, :socket, :reuseaddr, true) do
      adapt_role(role, %__MODULE__{state | socket: socket})
    end
  end

  @doc "Sends a message via the socket"
  def send(%__MODULE__{socket: socket} = state, data, _opts \\ []) do
    with :ok <- :socket.send(socket, Transport.encode(data)) do
      {:ok, state}
    end
  end

  @doc "Receives a message via the socket"
  def recv(%__MODULE__{socket: socket, buffer: buf} = state, opts \\ []) do
    {timeout, flags} = recv_opts(opts)
    with {:ok, received} <- :socket.recv(socket, 0, flags, timeout) do
      received = buf <> received

      case Transport.decode(received) do
        {:ok, msg, rest} ->
          {:ok, msg, %__MODULE__{state | buffer: rest}}

        {:more, _} ->
          recv(%__MODULE__{state | buffer: received}, opts)

        {:error, _} = err ->
          err
      end
    end
  end

  @doc "Receives a message via the socket, but does not block"
  def recv_nonblocking(%__MODULE__{socket: socket, buffer: buf} = state, opts \\ []) do
    with {:ok, received} <- :socket.recv(socket, 0, opts, :nowait) do
      received = buf <> received

      case Transport.decode(received) do
        {:ok, msg, rest} ->
          {:ok, msg, %__MODULE__{state | buffer: rest}}

        {:more, _} ->
          recv(%__MODULE__{state | buffer: received}, opts)

        {:error, _} = err ->
          err
      end
    end
  end

  @doc "Closes the socket"
  def close(%__MODULE__{socket: nil} = state), do: {:ok, state}

  def close(%__MODULE__{socket: socket} = state) do
    with :ok <- :socket.close(socket) do
      {:ok, state}
    end
  end

  defp recv_opts(opts), do: recv_opts(opts, :infinity, [])
  defp recv_opts([], timeout, flags), do: {timeout, flags}
  defp recv_opts([{:timeout, to} | rest], _timeout, flags) do
    recv_opts(rest, to, flags)
  end
  defp recv_opts([_ | rest], timeout, flags) do
    recv_opts(rest, timeout, flags)
  end

  defp adapt_role(:client, %__MODULE__{socket: socket, address: address} = state) do
    with {:ok, _p} <- :socket.bind(socket, :any),
         :ok <- :socket.connect(socket, address) do
      {:ok, state}
    else
      {:error, _} = err ->
        err
    end
  end

  defp adapt_role(:server, %__MODULE__{socket: socket} = state) do
    with {:ok, _p} <- :socket.bind(socket, state.address),
         :ok <- :socket.listen(socket) do
      {:ok, state}
    else
      {:error, _} = err ->
        err
    end
  end
end
