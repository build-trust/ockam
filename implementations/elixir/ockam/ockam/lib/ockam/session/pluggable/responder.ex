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

      ## TODO: match Ockam.Message?
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
        RoutingSession.switch_to_data_stage(response, options, handshake_state, state)

      {:ready, options, handshake_state} ->
        RoutingSession.switch_to_data_stage(options, handshake_state, state)

      {:next, response, handshake_state} ->
        case response do
          nil -> :ok
          %{} -> Ockam.Router.route(response)
        end

        {:ok, RoutingSession.update_handshake_state(state, handshake_state)}

      {:next, handshake_state} ->
        {:ok, RoutingSession.update_handshake_state(state, handshake_state)}

      {:error, err} ->
        {:stop, {:handshake_error, err}, state}
    end
  end
end
