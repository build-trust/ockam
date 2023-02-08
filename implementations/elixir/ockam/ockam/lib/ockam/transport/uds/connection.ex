defmodule Ockam.Transport.UDS.Connection do
  use GenServer

  alias Ockam.Router
  alias Ockam.Wire

  require Logger

  def start_link(socket) do
    GenServer.start_link(__MODULE__, socket)
  end

  def send_message(listener, message) do
    GenServer.cast(listener, {:send_message, message})
  end

  defstruct [:socket]

  @impl true
  def init(socket) do
    Logger.debug("Started connection handler")
    {:ok, %__MODULE__{socket: socket}}
  end

  @impl true
  def handle_info({:tcp, socket, data}, %__MODULE__{socket: socket} = state) do
    Logger.debug("Received data: #{inspect(data)}")

    case decode_and_send_to_router(data, state) do
      {:ok, state} ->
        {:noreply, state}

      {:error, error} ->
        {:stop, {:error, error}, state}
    end
  end

  defp decode_and_send_to_router(uds_message, state) do
    case Wire.decode(uds_message, :uds) do
      {:ok, decoded} ->
        with :ok <- Router.route(decoded) do
          {:ok, state}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end
end
