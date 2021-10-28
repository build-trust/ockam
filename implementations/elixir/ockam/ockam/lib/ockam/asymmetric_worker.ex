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
  `handle_other_message/2` - handle message received on a different address, other than `inner_address` or `address`
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

  @callback handle_other_message(message :: any(), state :: map()) ::
              {:ok, state :: map()}
              | {:error, reason :: any()}
              | {:stop, reason :: any(), state :: map()}

  ## TODO: remove that after refactoring Ockam.Worker to not call handle_message for non-messages
  @callback handle_non_message(message :: any(), state :: map()) ::
              {:ok, state :: map()}
              | {:error, reason :: any()}
              | {:stop, reason :: any(), state :: map()}

  defmacro __using__(_options) do
    quote do
      use Ockam.Worker

      alias Ockam.Message

      require Logger

      @behaviour Ockam.AsymmetricWorker

      @impl true
      def setup(options, state) do
        with {:ok, inner_address} <- register_inner_address(options) do
          inner_setup(options, Map.put(state, :inner_address, inner_address))
        end
      end

      @impl true
      def handle_message(message, state) do
        case message_type(message, state) do
          :inner ->
            handle_inner_message(message, state)

          :outer ->
            handle_outer_message(message, state)

          :other ->
            handle_other_message(message, state)

          :non_message ->
            handle_non_message(message, state)
        end
      end

      @doc false
      def register_inner_address(options) do
        case Keyword.get(options, :inner_address) do
          nil ->
            Ockam.Node.register_random_address()

          inner_address ->
            case Ockam.Node.register_address(inner_address) do
              :yes -> {:ok, inner_address}
              :no -> {:error, :inner_address_already_taken}
            end
        end
      end

      @doc false
      def message_type(%{onward_route: _} = message, state) do
        [me | _] = Message.onward_route(message)
        outer_address = state.address
        inner_address = state.inner_address

        case me do
          ^outer_address ->
            :outer

          ^inner_address ->
            :inner

          _other ->
            :other
        end
      end

      def message_type(_message, _state) do
        :non_message
      end

      def inner_setup(options, state), do: {:ok, state}

      def handle_other_message(message, state) do
        {:error, {:unknown_self_address, message, state}}
      end

      def handle_non_message(non_message, state) do
        {:error, {:not_ockam_message, non_message, state}}
      end

      defoverridable inner_setup: 2, handle_other_message: 2, handle_non_message: 2
    end
  end
end
