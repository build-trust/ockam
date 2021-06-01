defmodule Ockam.Worker do
  @moduledoc false

  @callback setup(options :: Keyword.t(), initial_state :: map()) ::
              {:ok, state :: map()} | {:error, reason :: any()}

  @callback handle_message(message :: Ockam.Message.t(), state :: map()) ::
              {:ok, state :: map()}
              | {:error, reason :: any()}
              | {:stop, reason :: any(), state :: map()}

  @callback address_prefix(options :: Keyword.t()) :: String.t()

  def call(worker, call, timeout \\ 5000)

  def call(worker, call, timeout) when is_pid(worker) do
    GenServer.call(worker, call, timeout)
  end

  def call(worker, call, timeout) do
    case Ockam.Node.whereis(worker) do
      nil -> raise "Worker #{inspect(worker)} not found"
      pid -> call(pid, call, timeout)
    end
  end

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

      ## Ignore match errors in handle_info when checking a result of handle_message
      ## handle_message definition may not return {:error, ...} and it shouldn't fail because of that
      @dialyzer {:no_match, handle_info: 2, handle_continue: 2}

      alias Ockam.Node
      alias Ockam.Router
      alias Ockam.Telemetry

      def create(options \\ []) when is_list(options) do
        address_prefix = Keyword.get(options, :address_prefix, address_prefix(options))

        options =
          Keyword.put_new_lazy(options, :address, fn ->
            Node.get_random_unregistered_address(address_prefix)
          end)

        case Node.start_supervised(__MODULE__, options) do
          {:ok, pid, worker} ->
            ## TODO: a better way to handle failing start
            try do
              :sys.get_state(pid)
              {:ok, worker}
            catch
              _type, err ->
                {:error, err}
            end

          error ->
            error
        end
      end

      def start_link(options) when is_list(options) do
        with {:ok, address} <- get_from_options(:address, options),
             {:ok, pid} <- start_link(address, options) do
          {:ok, pid, address}
        end
      end

      def start_link(address, options) do
        GenServer.start_link(__MODULE__, options, name: {:via, Node.process_registry(), address})
      end

      @doc false
      @impl true
      def init(options) do
        {:ok, options, {:continue, :post_init}}
      end

      @doc false
      @impl true
      def handle_info(%Ockam.Message{} = message, state) do
        metadata = %{message: message}
        start_time = Telemetry.emit_start_event([__MODULE__, :handle_message], metadata: metadata)

        ## TODO: error handling
        return_value = handle_message(message, state)

        metadata = Map.put(metadata, :return_value, return_value)
        Telemetry.emit_stop_event([__MODULE__, :handle_message], start_time, metadata: metadata)

        last_message_ts = System.os_time(:millisecond)

        case return_value do
          {:ok, returned_state} ->
            {:noreply, Map.put(returned_state, :last_message_ts, last_message_ts)}

          {:stop, reason, returned_state} ->
            {:stop, reason, returned_state}

          {:error, _reason} ->
            ## TODO: log error
            {:noreply, Map.put(state, :last_message_ts, last_message_ts)}
        end
      end

      @doc false
      @impl true
      def handle_continue(:post_init, options) do
        metadata = %{address: Keyword.get(options, :address)}
        start_time = Telemetry.emit_start_event([__MODULE__, :init], metadata: metadata)

        with {:ok, address} <- get_from_options(:address, options) do
          base_state = %{address: address, module: __MODULE__, last_message_ts: nil}
          return_value = setup(options, base_state)

          metadata = Map.put(metadata, :return_value, return_value)
          Telemetry.emit_stop_event([__MODULE__, :init], start_time, metadata: metadata)

          case return_value do
            {:ok, state} ->
              {:noreply, state}

            {:error, reason} ->
              {:stop, reason, base_state}
          end
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
      def address_prefix(_options), do: ""

      defoverridable setup: 2, address_prefix: 1
    end
  end
end
