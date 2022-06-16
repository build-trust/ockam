defmodule Ockam.Session.Separate.Initiator do
  @moduledoc """
  Initiator for `Ockam.Session.Separate`
  """
  use Ockam.AsymmetricWorker

  alias Ockam.AsymmetricWorker

  alias Ockam.Message
  alias Ockam.Router

  alias Ockam.Session.Separate.PreWorker

  @dialyzer {:nowarn_function, handle_inner_message: 2, handle_outer_message: 2}

  def get_stage(worker) do
    Ockam.Worker.call(worker, :get_stage)
  end

  def wait_for_session(worker, interval \\ 100, timeout \\ 5000)

  def wait_for_session(_worker, _interval, expire) when expire < 0 do
    {:error, :timeout}
  end

  def wait_for_session(worker, interval, timeout) do
    case get_stage(worker) do
      :data ->
        :ok

      _other ->
        :timer.sleep(interval)
        wait_for_session(worker, interval, timeout - interval)
    end
  end

  def create_and_wait(options, interval \\ 50, timeout \\ 5000) do
    with {:ok, address} <- create(options),
         :ok <- wait_for_session(address, interval, timeout) do
      {:ok, address}
    end
  end

  def get_data_worker(session_worker) do
    Ockam.Worker.call(session_worker, :get_data_worker)
  end

  @impl true
  def handle_call(:get_stage, _from, state) do
    {:reply, Map.get(state, :stage), state}
  end

  def handle_call(:get_data_worker, _from, state) do
    {:reply, Map.fetch!(state, :data_worker), state}
  end

  @impl true
  def inner_setup(options, state) do
    init_route = Keyword.fetch!(options, :init_route)

    ## rename to data_mod
    worker_mod = Keyword.fetch!(options, :worker_mod)
    worker_options = Keyword.get(options, :worker_options, [])

    handshake = Keyword.get(options, :handshake, Ockam.Session.Handshake.Default)
    handshake_options = Keyword.get(options, :handshake_options, [])

    {:ok, pre_worker} = PreWorker.create(worker_mod: worker_mod, worker_options: worker_options)
    {:ok, pre_worker_inner} = AsymmetricWorker.get_inner_address(pre_worker)

    hanshake_state = %{
      init_route: init_route,
      worker_address: pre_worker_inner,
      handshake_address: state.inner_address
    }

    with {:ok, hanshake_state} <- send_handshake(handshake, handshake_options, hanshake_state) do
      state =
        Map.merge(state, %{
          hanshake_state: hanshake_state,
          handshake: handshake,
          handshake_options: handshake_options,
          data_worker: pre_worker
        })

      {:ok, Map.put(state, :stage, :handshake)}
    end
  end

  def send_handshake(handshake, handshake_options, hanshake_state) do
    case handshake.init(handshake_options, hanshake_state) do
      {:next, handshake_msg, hanshake_state} ->
        Router.route(handshake_msg)
        {:ok, hanshake_state}

      {:next, handshake_state} ->
        {:ok, handshake_state}

      {:error, error} ->
        {:error, error}
    end
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

  def handle_message(message, %{stage: :data} = state) do
    case message_type(message, state) do
      :outer ->
        data_worker = Map.fetch!(state, :data_worker)
        [_me | onward_route] = Message.onward_route(message)

        ## TODO: forward_through
        Router.route(Message.set_onward_route(message, [data_worker | onward_route]))

      _other ->
        Logger.warn("Ignoring message in data stage: #{inspect(message)}. Not implemented")
    end

    {:ok, state}
  end

  def handle_handshake_message(message, state) do
    handshake = Map.fetch!(state, :handshake)
    handshake_options = Map.fetch!(state, :handshake_options)
    hanshake_state = Map.fetch!(state, :hanshake_state)

    case handshake.handle_initiator(handshake_options, message, hanshake_state) do
      {:ready, options, hanshake_state} ->
        switch_to_data_stage(options, hanshake_state, state)

      ## TODO: handle {:next, message, state} and {:ready, message, options, state}
      {:error, err} ->
        ## TODO: error handling in Ockam.Worker
        {:error, err}
    end
  end

  def switch_to_data_stage(options, hanshake_state, state) do
    pre_worker = Map.fetch!(state, :data_worker)

    case PreWorker.start(pre_worker, options) do
      :ok ->
        Logger.info("Worker started: #{inspect(pre_worker)}")
        {:ok, Map.merge(state, %{hanshake_state: hanshake_state, stage: :data})}

      {:error, err} ->
        {:stop, {:cannot_start_data_worker, {:error, err}, options, hanshake_state}, state}
    end
  end
end
