defmodule Ockam.AsymmetricWorker do
  @moduledoc """
  Ockam.Worker with two addresses.

  On start registers an additional `inner_address`

  Usage:

  `use Ockam.AsymmetricWorker`

  Callbacks:

  `inner_setup/2` - same as `Ockam.Worker.setup/2`, but `state` would have already registered `inner_address`
  `handle_inner_message/2` - handle message received on `inner_address`
  `handle_outer_message/2` - handle message received on `address`
  """

  @callback inner_setup(Keyword.t(), map()) :: {:ok, state :: map()} | {:error, reason :: any()}
  @callback handle_inner_message(message :: any(), state :: map()) ::
              {:ok, state :: map()}
              | {:error, reason :: any()}
              | {:stop, reason :: any(), state :: map()}

  @callback handle_outer_message(message :: any(), state :: map()) ::
              {:ok, state :: map()}
              | {:error, reason :: any()}
              | {:stop, reason :: any(), state :: map()}

  defmacro __using__(_options) do
    quote do
      use Ockam.Worker

      alias Ockam.Message

      @behaviour Ockam.AsymmetricWorker

      @impl true
      def setup(options, state) do
        state = register_inner_address(state)

        inner_setup(options, state)
      end

      @impl true
      def handle_message(message, state) do
        case message_type(message, state) do
          :inner ->
            handle_inner_message(message, state)

          :outer ->
            handle_outer_message(message, state)
        end
      end

      @doc false
      def register_inner_address(state) do
        {:ok, inner_address} = Ockam.Node.register_random_address()
        Map.put(state, :inner_address, inner_address)
      end

      @doc false
      def message_type(message, state) do
        [me | _] = Message.onward_route(message)
        outer_address = state.address
        inner_address = state.inner_address

        case me do
          ^outer_address ->
            :outer

          ^inner_address ->
            :inner
        end
      end

      def inner_setup(options, state), do: {:ok, state}

      defoverridable inner_setup: 2
    end
  end
end
