## Single handshake responder
defmodule Ockam.Session.Pluggable.Responder do
  @moduledoc """
  Routing session responder

  If :init_message is present in the options - processes the message,
  otherwise waits for it in :handshake stage

  On processing the handshake calls `handshake.handle_responder/1`, which
  generates handshake response message and options

  Starts the data worker with worker_options merged with
  the options from `handshake.handle_responder/1`

  If worker started successfully, sends the handshake response
  and moves to the :data stage

  All messages in :data stage are processed with the data worker module

  Options:

  `worker_mod` - data worker module
  `worker_options` - data worker options, defaults to []

  `handshake` - handshake module (defaults to `Ockam.Session.Handshake.Default`)
  `handshake_options` - options for handshake module, defaults to []

  `init_message` - optional init message
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Message
  alias Ockam.Session.Pluggable, as: RoutingSession

  require Logger

  @dialyzer {:nowarn_function, handle_inner_message: 2, handle_outer_message: 2}

  @impl true
  def address_prefix(_options), do: "S_R_"

  @impl true
  def inner_setup(options, state) do
    base_state = state
    worker_mod = Keyword.fetch!(options, :worker_mod)
    worker_options = Keyword.get(options, :worker_options, [])

    handshake = Keyword.get(options, :handshake, Ockam.Session.Handshake.Default)
    handshake_options = Keyword.get(options, :handshake_options, [])

    handshake_state = %{
      worker_address: state.inner_address,
      handshake_address: state.inner_address
    }

    state =
      Map.merge(state, %{
        worker_mod: worker_mod,
        worker_options: worker_options,
        base_state: base_state,
        stage: :handshake,
        handshake: handshake,
        handshake_options: handshake_options,
        handshake_state: handshake_state
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
    RoutingSession.handle_data_message(message, state)
  end

  def handle_handshake_message(message, state) do
    handshake = Map.fetch!(state, :handshake)
    handshake_options = Map.fetch!(state, :handshake_options)
    handshake_state = Map.fetch!(state, :handshake_state)

    case handshake.handle_responder(handshake_options, message, handshake_state) do
      {:ready, response, options, handshake_state} ->
        switch_to_data_stage(response, options, handshake_state, state)

      {:ready, options, handshake_state} ->
        switch_to_data_stage(options, handshake_state, state)

      {:next, response, handshake_state} ->
        send_handshake_response(response)
        {:ok, Map.put(state, :handshake_state, handshake_state)}

      {:next, handshake_state} ->
        {:ok, Map.put(state, :handshake_state, handshake_state)}

      {:error, err} ->
        {:error, err}
    end
  end

  defp switch_to_data_stage(response \\ nil, handshake_options, handshake_state, state) do
    worker_mod = Map.fetch!(state, :worker_mod)
    worker_options = Map.fetch!(state, :worker_options)
    base_state = Map.fetch!(state, :base_state)

    options = Keyword.merge(worker_options, handshake_options)

    case worker_mod.setup(options, base_state) do
      {:ok, data_state} ->
        send_handshake_response(response)

        {:ok,
         Map.merge(state, %{
           data_state: data_state,
           handshake_state: handshake_state,
           stage: :data
         })}

      {:error, err} ->
        Logger.error(
          "Error starting responder data module: #{worker_mod}, reason: #{inspect(err)}"
        )

        ## TODO: should we send handshake error?
        {:error, err}
    end
  end

  def send_handshake_response(response) do
    case response do
      nil ->
        :ok

      %{} ->
        Logger.info("Send handshake #{inspect(response)}")
        Ockam.Router.route(response)
    end
  end
end
