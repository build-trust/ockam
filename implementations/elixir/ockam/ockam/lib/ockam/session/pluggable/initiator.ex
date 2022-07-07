## First option: non-buffering session, messages are dropped if the session is not ready yet
defmodule Ockam.Session.Pluggable.Initiator do
  @moduledoc """
  Simple routing session initiator

  Upon starting, uses Handshake.init to generate a handshake message
  and send it to init_route.
  Initial stage is :handshake, in this stage waits for a handshake response

  After receiving a handshake response, runs Handshake.handle_initiator
  and starts the data worker on the same process and moves to the :data stage

  Data worker is started with `worker_options` merged with
  the options from handle_initiator

  In the :data stage processes all messages with the data worker module

  Options:

  `init_route` - route to responder (or spawner)

  `worker_mod` - data worker module
  `worker_options` - data worker options

  `handshake` - handshake module (defaults to `Ockam.Session.Handshake.Default`)
  `handshake_options` - options for handshake module
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Message
  alias Ockam.Router

  alias Ockam.Session.Pluggable, as: RoutingSession

  require Logger

  @dialyzer {:nowarn_function, handle_inner_message: 2, handle_outer_message: 2}

  def get_stage(worker) do
    Ockam.Worker.call(worker, :get_stage)
  end

  def wait_for_session(worker, interval \\ 100, timeout \\ 5000)

  def wait_for_session(worker, _interval, expire) when is_integer(expire) and expire < 0 do
    {:error, {:timeout, worker}}
  end

  def wait_for_session(worker, interval, timeout) when is_integer(timeout) do
    case get_stage(worker) do
      :data ->
        :ok

      :handshake ->
        :timer.sleep(interval)
        wait_for_session(worker, interval, timeout - interval)
    end
  end

  def wait_for_session(worker, interval, :infinity) do
    case get_stage(worker) do
      :data ->
        :ok

      :handshake ->
        :timer.sleep(interval)
        wait_for_session(worker, interval, :infinity)
    end
  end

  def create_and_wait(options, interval \\ 50, timeout \\ 5000) do
    with {:ok, address} <- create(options),
         :ok <- wait_for_session(address, interval, timeout) do
      {:ok, address}
    end
  catch
    type, reason ->
      ## TODO: match Worker not found error?
      {:error, {type, reason}}
  end

  @impl true
  def address_prefix(_options), do: "S_I_"

  @impl true
  def inner_setup(options, state) do
    ## TODO: should init_route be in the handshake options?
    init_route = Keyword.fetch!(options, :init_route)

    ## rename to data_mod
    worker_mod = Keyword.fetch!(options, :worker_mod)
    worker_options = Keyword.get(options, :worker_options, [])

    base_state = Map.put(state, :module, worker_mod)

    handshake = Keyword.get(options, :handshake, Ockam.Session.Handshake.Default)
    handshake_options = Keyword.get(options, :handshake_options, [])

    handshake_state = %{
      init_route: init_route,
      worker_address: state.inner_address,
      handshake_address: state.inner_address
    }

    state =
      Map.merge(state, %{
        worker_mod: worker_mod,
        worker_options: worker_options,
        base_state: base_state,
        handshake: handshake,
        handshake_options: handshake_options,
        stage: :handshake
      })

    with {:ok, handshake_state} <- init_handshake(handshake, handshake_options, handshake_state) do
      state = RoutingSession.update_handshake_state(state, handshake_state)
      {:ok, state}
    end
  end

  def init_handshake(handshake, handshake_options, handshake_state) do
    case handshake.init(handshake_options, handshake_state) do
      {:next, handshake_msg, handshake_state} ->
        maybe_send_message(handshake_msg)
        {:ok, handshake_state}

      {:next, handshake_state} ->
        {:ok, handshake_state}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @impl true
  def handle_call(:get_stage, _from, state) do
    {:reply, Map.get(state, :stage), state}
  end

  @impl true
  def handle_message(message, %{stage: :handshake} = state) do
    case message_type(message, state) do
      :inner ->
        handle_handshake_message(message, state)

      _other ->
        Logger.info("Ignoring non-inner message in handshake stage: #{inspect(message)}")
        {:ok, state}
    end
  end

  def handle_message(message, %{stage: :data, data_state: _, worker_mod: _} = state) do
    RoutingSession.handle_data_message(message, state)
  end

  def handle_handshake_message(message, state) do
    handshake = Map.fetch!(state, :handshake)
    handshake_options = Map.fetch!(state, :handshake_options)
    handshake_state = Map.fetch!(state, :handshake_state)

    case handshake.handle_initiator(handshake_options, message, handshake_state) do
      {:ready, options, handshake_state} ->
        RoutingSession.switch_to_data_stage(options, handshake_state, state)

      {:ready, message, options, handshake_state} ->
        RoutingSession.switch_to_data_stage(message, options, handshake_state, state)

      {:next, message, handshake_state} ->
        maybe_send_message(message)
        {:ok, RoutingSession.update_handshake_state(state, handshake_state)}

      {:next, handshake_state} ->
        {:ok, RoutingSession.update_handshake_state(state, handshake_state)}

      {:error, err} ->
        ## TODO: should we return :shutdown error here?
        ## Non-shutdown error will cause a resstart
        {:stop, {:handshake_error, err}, state}
    end
  end

  def maybe_send_message(nil) do
    :ok
  end

  def maybe_send_message(%{} = message) do
    Logger.debug("Sending handshake #{inspect(message)}")
    Router.route(message)
  end
end
