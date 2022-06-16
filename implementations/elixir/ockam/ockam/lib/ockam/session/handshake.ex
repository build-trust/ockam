defmodule Ockam.Session.Handshake do
  @moduledoc """
  Session handshake behaviour.

  Used in `Ockam.Session.Pluggable` and `Ockam.Session.Separate` modules

  """
  @type message() :: Ockam.Router.message()

  @typedoc """
  State passed to the callbacks, can be modified, but following fields are required:

  `init_route` - (initiator only) session initial route to send initial handshake to
  `worker_address` - inner address of the data worker
  `handshake_address` - inner address of the handshake worker

  For pluggable session, `worker_address` and `handshake_address` are the same
  """
  @type handshake_state() :: %{
          :init_route => Ockam.Address.route(),
          :worker_address => Ockam.Address.t(),
          :handshake_address => Ockam.Address.t(),
          any() => any()
        }

  @doc """
  Create initial handshake message using handshake options and state
  Called by session initiator
  """
  @callback init(options :: Keyword.t(), handshake_state()) ::
              {:next, handshake_state()} | {:next, message(), handshake_state()} | {:error, any()}

  ## TODO: error result
  @doc """
  Handle handshake message on session initiator

  Arguments:
  `handshake_options` - options set in `handshake_options` for session worker
  `message` - handshake message received
  `handshake_state` - additional worker state, see type `handshake_state/0`

  Returns:
  `{:next, handshake_state()}` - wait for the next handshake message
  `{:next, message, handshake_state()}` - send a handshake message and wait for next one
  `{:ready, options, handshake_state()}` - start data worker with options and switch to data mode
  `{:ready, message, options, handshake_state()}` - start data worker and send handshake message
  """
  @callback handle_initiator(
              handshake_options :: Keyword.t(),
              message(),
              handshake_state()
            ) ::
              {:next, handshake_state()}
              | {:next, message(), handshake_state()}
              | {:ready, worker_options :: Keyword.t(), handshake_state()}
              | {:ready, message(), worker_options :: Keyword.t(), handshake_state()}
              | {:error, reason :: any()}

  @doc """
  Handle handshake on session responder

  see `handle_initiator/3`
  """
  @callback handle_responder(
              handshake_options :: Keyword.t(),
              message :: message(),
              state :: handshake_state()
            ) ::
              {:next, handshake_state()}
              | {:next, message(), handshake_state()}
              | {:ready, worker_options :: Keyword.t(), handshake_state()}
              | {:ready, message(), worker_options :: Keyword.t(), handshake_state()}
              | {:error, reason :: any()}
end
