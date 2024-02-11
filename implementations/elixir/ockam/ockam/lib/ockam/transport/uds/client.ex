defmodule Ockam.Transport.UDS.Client do
  @moduledoc false
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Wire
  alias Ockam.Transport.TCP

  require Logger

  @impl true
  def address_prefix(_options), do: "UDS_C_"

  @impl true
  def setup(options, state) do
    path = Keyword.fetch!(options, :path)

    case :gen_tcp.connect({:local, path}, 0, [:binary, active: true, reuseaddr: true]) do
      {:ok, socket} ->
        :gen_tcp.controlling_process(socket, self())

        state =
          Map.merge(state, %{
            socket: socket,
            path: path
          })

        {:ok, state}

      {:error, reason} ->
        {:stop, reason}
    end
  end

  @impl true
  def handle_info({:tcp, socket, data}, %{socket: socket} = state) do
    ## TODO: send/receive message in multiple TCP packets
    case Wire.decode(data, :tcp) do
      {:ok, message} ->
        forwarded_message =
          message
          |> Message.trace(state.address)

        Ockam.Router.route(forwarded_message)

      {:error, %Wire.DecodeError{} = e} ->
        raise e

      e ->
        raise e
    end

    {:noreply, state}
  end

  def handle_info({:tcp_closed, _}, state) do
    {:stop, :normal, state}
  end

  def handle_info({:tcp_error, _}, state) do
    {:stop, :normal, state}
  end

  @impl true
  def handle_message(%{payload: _payload} = message, state) do
    with :ok <- encode_and_send_over_uds(message, state) do
      {:ok, state}
    end
  end

  defp encode_and_send_over_uds(message, state) do
    forwarded_message = Message.forward(message)

    with {:ok, encoded_message} <- Wire.encode(forwarded_message) do
      ## TODO: send/receive message in multiple TCP packets
      case byte_size(encoded_message) <= TCP.packed_size_limit() do
        true ->
          send_over_tcp(encoded_message, state)

        false ->
          Logger.error("Message to big for TCP: #{inspect(message)}")
          {:error, {:message_too_big, message}}
      end
    end
  end

  defp send_over_tcp(data, %{socket: socket}) do
    :gen_tcp.send(socket, data)
  end
end
