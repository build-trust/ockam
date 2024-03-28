defmodule Ockam.Transport.Portal.Interceptor do
  @moduledoc """
  Interceptor worker for portals.

  Can be inserted in the route between inlet and outlet.
  Messages on outer address are coming from the inlet, messages on inner address
  are coming from the outlet.

  Supports `:init_message` handling from `Ockam.Session.Spawner`

  Supports message reconstruction from multiple tunnel messages (batching).

  Messages are parsed and processed by behaviour functions
  For payload messages:
  `handle_outer_payload`
  `handle_inner_payload`

  For signal messages (:ping, :pong, :disconnect):
  `handle_outer_signal`
  `handle_inner_signal`

  :disconnect stops the interceptor worker

  Other messages are forwarded as is.

  `outer` and `inner` depends on which direction message flow created the interceptor,
  usually it's configured in the route of the inlet, so `outer` is inlet and `inner` is outlet.

  Options:
  :interceptor_mod - module implementing interceptor behaviour
  :interceptor_options - options to initialize interceptor with
  :init_message - message from `Ockam.Session.Spawner`
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Message
  alias Ockam.Transport.Portal.TunnelProtocol
  alias Ockam.Worker

  ## TODO: enforce that on inlet/outlet level
  @max_payload_size 48 * 1024

  @doc """
  Modify interceptor worker state.
  """
  @callback setup(options :: Keyword.t(), state :: map()) ::
              {:ok, state :: map()} | {:error, reason :: any()}

  @doc """
  Process intercepted payload from outer worker.
  Returns list of payloads to forward, which can be used to buffer messages of different size.
  This is done because tcp portal batches tcp packets.
  Returns:
   - {:ok, payloads, state} - forward 0 or more payloads
   - {:stop, reason, payloads, state} - forward 0 or more payloads and terminate the interceptor
  """
  @callback handle_outer_payload(payload :: binary(), state :: map()) ::
              {:ok, payloads :: [binary()], state :: map()}
              | {:stop, reason :: any(), payloads :: [binary()], state :: map()}
  @doc """
  Process intercepted payload from inner worker.
  Returns list of payloads to forward, which can be used to buffer messages of different size.
  This is done because tcp portal batches tcp packets.
  Returns:
   - {:ok, payloads, state} - forward 0 or more payloads
   - {:stop, reason, payloads, state} - forward 0 or more payloads and terminate the interceptor
  """
  @callback handle_inner_payload(payload :: binary(), state :: map()) ::
              {:ok, payloads :: [binary()], state :: map()}
              | {:stop, reason :: any(), payload :: [binary()], state :: map()}
  @doc """
  Process intercepted signal from outer worker (:ping, :pong, :disconnect).
  Returns:
   - {:ok, state} - will forward original signal
   - {:error, reason} - will not forward the signal
  """
  @callback handle_outer_signal(signal :: any(), state :: map()) ::
              {:ok, state :: map()}
              | {:stop, reason :: any(), state :: map()}
  @doc """
  Process intercepted signal from inner worker (:ping, :pong, :disconnect).
  Returns:
   - {:ok, state} - will forward original signal
   - {:error, reason} - will not forward the signal
  """
  @callback handle_inner_signal(signal :: any(), state :: map()) ::
              {:ok, state :: map()}
              | {:stop, reason :: any(), state :: map()}

  @impl true
  def inner_setup(options, state) do
    interceptor_mod = Keyword.fetch!(options, :interceptor_mod)
    interceptor_options = Keyword.get(options, :interceptor_options, [])

    state =
      Map.merge(
        state,
        %{
          inner_incoming_packet_counter: 0xFFFF,
          inner_outgoing_packet_counter: 0xFFFF,
          outer_incoming_packet_counter: 0xFFFF,
          outer_outgoing_packet_counter: 0xFFFF
        }
      )

    case interceptor_mod.setup(interceptor_options, state) do
      {:ok, state} ->
        state = Map.put(state, :interceptor_mod, interceptor_mod)

        case Keyword.fetch(options, :init_message) do
          {:ok, message} ->
            ## Interceptor is spawned by Ockam.Session.Spawner
            handle_outer_message(message, state)

          :error ->
            {:ok, state}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  @impl true
  def handle_outer_message(message, state) do
    handle_message(:outer, message, state)
  end

  @impl true
  def handle_inner_message(message, state) do
    handle_message(:inner, message, state)
  end

  defp handle_message(direction, %Message{} = message, state) do
    case handle_tunnel_message(direction, message, state) do
      {:ok, payloads, state} ->
        ## Currently sending all payloads to the same route,
        ## even if they were generated from other messages.
        ## TODO: only process messages from inlet and outlet (known) routes
        ## and/or known inlet and outlet identities
        forward_payloads(direction, message, payloads, state)
        {:ok, state}

      {:stop, reason, payloads, state} ->
        forward_payloads(direction, message, payloads, state)
        send_disconnect(state)
        {:stop, reason, state}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp send_disconnect(state) do
    case Map.fetch(state, :ping_route) do
      {:ok, route} ->
        Worker.route(TunnelProtocol.encode(:disconnect), route, [], %{}, state)

      :error ->
        :ok
    end

    case Map.fetch(state, :pong_route) do
      {:ok, route} ->
        Worker.route(TunnelProtocol.encode(:disconnect), route, [], %{}, state)

      :error ->
        :ok
    end
  end

  defp forward_payloads(direction, message, payloads, state) do
    Enum.each(payloads, fn payload ->
      :ok = forward_message(direction, message, payload, state)
    end)
  end

  defp forward_message(direction, message, new_payload, state) do
    return_address =
      case direction do
        :outer -> state.inner_address
        :inner -> state.address
      end

    ## TODO: figure out if we need to forward message local metadata
    message
    |> Message.forward()
    |> Message.trace(return_address)
    |> Message.set_payload(new_payload)
    |> Message.set_local_metadata(%{})
    |> Worker.route(state)
  end

  defp handle_tunnel_message(type, %Message{payload: payload} = message, state) do
    case TunnelProtocol.decode(payload) do
      {:ok, {:payload, {tunnel_payload, packet_counter}}} ->
        intercept_tunnel_payload_packet_counter(type, tunnel_payload, packet_counter, state)

      {:ok, signal} ->
        case intercept_tunnel_signal(type, signal, state) do
          {:ok, state} ->
            handle_tunnel_signal(signal, message, state)

          {:stop, reason, state} ->
            {:stop, reason, [payload], state}
        end

      {:error, reason} ->
        ## TODO: should we return error here?
        Logger.warning("Cannot parse tunnel message #{inspect(reason)}, ignoring")
        {:ok, payload, state}
    end
  end

  defp intercept_tunnel_payload_packet_counter(type, payload, msg_packet_counter, state) do
    if msg_packet_counter != :undefined do
      packet_counter =
        case type do
          :outer -> Map.get(state, :outer_incoming_packet_counter)
          :inner -> Map.get(state, :inner_incoming_packet_counter)
        end

      packet_counter = increment_packet_counter(packet_counter)

      if packet_counter == msg_packet_counter do
        state =
          case type do
            :outer -> Map.put(state, :outer_incoming_packet_counter, packet_counter)
            :inner -> Map.put(state, :inner_incoming_packet_counter, packet_counter)
          end

        intercept_tunnel_payload(type, payload, state)
      else
        Logger.warning(
          "Packet counter mismatch, expected #{packet_counter}, got #{msg_packet_counter}"
        )

        {:stop, :packet_counter_mismatch, state}
      end
    else
      intercept_tunnel_payload(type, payload, state)
    end
  end

  defp intercept_tunnel_payload(type, payload, %{interceptor_mod: interceptor_mod} = state) do
    mod_return =
      case type do
        :outer -> interceptor_mod.handle_outer_payload(payload, state)
        :inner -> interceptor_mod.handle_inner_payload(payload, state)
      end

    case mod_return do
      {:ok, payloads, state} ->
        {encoded_payloads, state} = encode_payloads(type, payloads, state)
        {:ok, encoded_payloads, state}

      ## TODO: send disconnect to both sides on exit?
      {:stop, reason, payloads, state} ->
        {encoded_payloads, state} = encode_payloads(type, payloads, state)
        {:stop, reason, encoded_payloads, state}
    end
  end

  defp encode_payloads(type, payloads, state) do
    packet_counter =
      case type do
        :outer -> Map.get(state, :outer_outgoing_packet_counter)
        :inner -> Map.get(state, :inner_outgoing_packet_counter)
      end

    ## Each payload may result in 1 or more tunnel payloads
    {encoded_payloads, packet_counter} =
      Enum.flat_map_reduce(payloads, packet_counter, fn payload, packet_counter ->
        case byte_size(payload) do
          small when small <= @max_payload_size ->
            packet_counter = increment_packet_counter(packet_counter)

            {
              [TunnelProtocol.encode({:payload, {payload, packet_counter}})],
              packet_counter
            }

          large ->
            chunks = chunk_payload(large, @max_payload_size)

            Enum.map_reduce(chunks, packet_counter, fn payload ->
              packet_counter = increment_packet_counter(packet_counter)

              {
                TunnelProtocol.encode({:payload, {payload, packet_counter}}),
                packet_counter
              }
            end)
        end
      end)

    state =
      case type do
        :outer -> Map.put(state, :outer_outgoing_packet_counter, packet_counter)
        :inner -> Map.put(state, :inner_outgoing_packet_counter, packet_counter)
      end

    {encoded_payloads, state}
  end

  defp chunk_payload(payload, max_size) when byte_size(payload) >= max_size do
    <<chunk::binary-size(max_size), rest::binary>> = payload
    [chunk | chunk_payload(rest, max_size)]
  end

  defp chunk_payload(<<>>, _maz_size) do
    []
  end

  defp chunk_payload(payload, max_size) when byte_size(payload) < max_size do
    [payload]
  end

  defp intercept_tunnel_signal(type, signal, %{interceptor_mod: interceptor_mod} = state) do
    case type do
      :outer -> interceptor_mod.handle_outer_signal(signal, state)
      :inner -> interceptor_mod.handle_inner_signal(signal, state)
    end
  end

  defp handle_tunnel_signal(signal, %Message{payload: payload} = message, state) do
    case signal do
      :disconnect ->
        ## We will send disconnect on stop handling
        {:stop, :normal, [], state}

      :ping ->
        {:ok, [payload], Map.put(state, :ping_route, Message.return_route(message))}

      :pong ->
        {:ok, [payload], Map.put(state, :pong_route, Message.return_route(message))}
    end
  end

  defp increment_packet_counter(packet_counter) do
    # 0xFFFF is the maximum value for a u16
    if packet_counter == 0xFFFF do
      0
    else
      packet_counter + 1
    end
  end
end
