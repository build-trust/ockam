defmodule Ockam.Session.Separate.Responder do
  @moduledoc """
  Responder for `Ockam.Session.Separate`
  """
  use Ockam.AsymmetricWorker

  alias Ockam.AsymmetricWorker

  alias Ockam.Message

  alias Ockam.Session.Separate.PreWorker

  require Logger

  @dialyzer {:nowarn_function, handle_inner_message: 2, handle_outer_message: 2}

  def get_data_worker(session_worker) do
    Ockam.Worker.call(session_worker, :get_data_worker)
  end

  @impl true
  def address_prefix(_options), do: "S_S_R_"

  @impl true
  def inner_setup(options, state) do
    worker_mod = Keyword.fetch!(options, :worker_mod)
    worker_options = Keyword.get(options, :worker_options, [])

    handshake = Keyword.get(options, :handshake, Ockam.Session.Handshake.Default)
    handshake_options = Keyword.get(options, :handshake_options, [])

    {:ok, pre_worker} = PreWorker.create(worker_mod: worker_mod, worker_options: worker_options)
    {:ok, pre_worker_inner} = AsymmetricWorker.get_inner_address(pre_worker)

    hanshake_state = %{
      worker_address: pre_worker_inner,
      handshake_address: state.inner_address
    }

    state =
      Map.merge(state, %{
        stage: :handshake,
        hanshake_state: hanshake_state,
        handshake: handshake,
        handshake_options: handshake_options,
        data_worker: pre_worker
      })

    case Keyword.get(options, :init_message) do
      nil ->
        ## Stay in the handshake stage, wait for init message
        {:ok, state}

      %{payload: _} = message ->
        handle_handshake_message(message, state)
    end
  end

  @impl true
  def handle_call(:get_data_worker, _from, state) do
    {:reply, Map.fetch!(state, :data_worker), state}
  end

  @impl true
  def handle_message(message, %{stage: :handshake} = state) do
    case message_type(message, state) do
      :inner ->
        handle_handshake_message(message, state)

      _other ->
        ## TODO: buffering option?
        Logger.debug("Ignoring non-inner message #{inspect(message)} in handshake stage")
        {:ok, state}
    end
  end

  def handle_message(message, %{stage: :data} = state) do
    Logger.debug("Ignoring message in data stage: #{inspect(message)}. Not implemented")
    {:ok, state}
  end

  def handle_handshake_message(message, state) do
    handshake = Map.fetch!(state, :handshake)
    handshake_options = Map.fetch!(state, :handshake_options)

    hanshake_state = Map.fetch!(state, :hanshake_state)

    case handshake.handle_responder(handshake_options, message, hanshake_state) do
      {:ready, response, options, hanshake_state} ->
        switch_to_data_stage(response, options, hanshake_state, state)

      ## TODO: handle {:next, message, state} and {:ready, options, state}
      {:error, err} ->
        {:error, err}
    end
  end

  defp switch_to_data_stage(response, handshake_options, hanshake_state, state) do
    pre_worker = Map.fetch!(state, :data_worker)

    case PreWorker.start(pre_worker, handshake_options) do
      :ok ->
        Logger.info("handshake response: #{inspect(response)}")
        send_handshake_response(response)
        {:ok, Map.merge(state, %{hanshake_state: hanshake_state, stage: :data})}

      {:error, err} ->
        worker_mod = Map.fetch!(state, :worker_mod)

        Logger.error(
          "Error starting responder data module: #{worker_mod}, reason: #{inspect(err)}"
        )

        ## TODO: should we send handshake error?
        {:error, err}
    end
  end

  def send_handshake_response(response) do
    Ockam.Router.route(response)
  end
end
