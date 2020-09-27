defmodule Ockam.Transport.UDP.Server do
  @moduledoc false

  use GenServer

  alias Ockam.Routing

  require Logger

  @default_ip {127, 0, 0, 1}
  @default_port 5000

  # Starts the transport process linked to the current process
  @doc false
  def start_link(options) do
    GenServer.start_link(__MODULE__, options, name: {:via, Routing, {0, 1}})
  end

  @doc false
  @impl true
  def init(options) do
    options = Enum.into(options, %{})
    options = Map.put_new(options, :ip, @default_ip)
    options = Map.put_new(options, :port, @default_port)

    udp_open_options = [:binary, :inet, {:ip, options.ip}, {:active, true}]

    with {:ok, socket} <- :gen_udp.open(options.port, udp_open_options) do
      state = Map.put(options, :socket, socket)

      Logger.info("Starting: #{inspect({__MODULE__, state})}")
      {:ok, state}
    end
  end

  @doc false
  @impl true

  def handle_info({:udp, _socket, _from_ip, _from_port, _packet} = incoming, state) do
    Logger.info("Incoming: #{inspect(incoming)}")
    {:noreply, state}
  rescue
    error ->
      Logger.error("Incoming error: #{inspect({error, incoming, state})}")
      {:noreply, state}
  end

  def handle_info(_message, state) do
    # :ok = :gen_udp.send(state.socket, to_ip, to_port, encoded)
    {:noreply, state}
  end
end
