defmodule Ockam.Worker do
  @moduledoc false

  @callback setup(options :: Keyword.t(), initial_state :: %{}) ::
              {:ok, state :: %{}} | {:error, reason :: any()}

  @callback handle_message(message :: any(), state :: %{}) ::
              {:ok, state :: %{}} | {:error, reason :: any()}

  defmacro __using__(_options) do
    quote do
      # use GenServer, makes this module a GenServer.
      #
      # Among other things, it adds the `child_spec/1` function which returns a
      # specification to start this module under a supervisor. When this module is
      # added to a supervisor, the supervisor calls child_spec to figure out the
      # specification that should be used.
      #
      # See the "Child specification" section in the `Supervisor` module for more
      # detailed information.
      #
      # The `@doc` annotation immediately preceding `use GenServer` below
      # is attached to the generated `child_spec/1` function. Since we don't
      # want `child_spec/1` in our Transport module docs, `@doc false` is set here.

      @doc false
      use GenServer

      @behaviour Ockam.Worker

      alias Ockam.Node
      alias Ockam.Router
      alias Ockam.Telemetry

      defstruct [:address]

      @doc false
      def create(options) when is_list(options) do
        options = Keyword.put_new_lazy(options, :address, &Node.get_random_unregistered_address/0)

        case Node.start_supervised(__MODULE__, options) do
          {:ok, _pid, worker} -> {:ok, worker}
          error -> error
        end
      end

      @doc false
      def start_link(options) when is_list(options) do
        with {:ok, address} <- get_from_options(:address, options),
             {:ok, pid} <- start(address, options) do
          {:ok, pid, address}
        end
      end

      defp start(address, options) do
        GenServer.start_link(__MODULE__, options, name: {:via, Node.process_registry(), address})
      end

      @doc false
      @impl true
      def init(options) do
        with {:ok, address} <- get_from_options(:address, options) do
          setup(options, %{address: address})
        end
      end

      @doc false
      @impl true
      def handle_info(message, state) do
        metadata = %{message: message}
        start_time = Telemetry.emit_start_event([__MODULE__, :handle_message], metadata: metadata)

        return_value = handle_message(message, state)

        with {:ok, instruction, state} <- return_value,
             {:ok, state} <- handle_instruction(instruction, state) do
          metadata = Map.put(metadata, :return_value, return_value)
          Telemetry.emit_stop_event([__MODULE__, :handle_message], start_time, metadata: metadata)
        end

        {:noreply, state}
      end

      defp handle_instruction({:route, messages}, state) when is_list(messages) do
        Enum.map(messages, &send_to_router/1)
      end

      defp handle_instruction({:route, message}, state) do
        case send_to_router(message) do
          :ok -> {:ok, state}
          {:error, reason} -> {:error, reason}
        end
      end

      defp handle_instruction(nil, state), do: {:ok, state}

      defp handle_instruction(instruction, state),
        do: {:error, {:unknown_instruction, instruction}}

      defp send_to_router(message) do
        case Router.route(message) do
          :ok -> {:ok, message}
          {:error, reason} -> {:error, {:could_not_route, {message, reason}}}
        end
      end

      @doc false
      def get_from_options(key, options) do
        case Keyword.get(options, key) do
          nil -> {:error, {:option_is_nil, key}}
          value -> {:ok, value}
        end
      end

      @doc false
      def setup(_options, state), do: {:ok, state}

      @doc false
      def handle_message(message, state), do: {:ok, state}

      defoverridable setup: 2, handle_message: 2
    end
  end
end
