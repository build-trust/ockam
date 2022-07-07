defmodule Ockam.Worker do
  @moduledoc false

  alias Ockam.Node
  alias Ockam.Telemetry
  alias Ockam.Worker.Authorization

  require Logger

  @callback setup(options :: Keyword.t(), initial_state :: map()) ::
              {:ok, state :: map()} | {:error, reason :: any()}

  @callback handle_message(message :: Ockam.Message.t(), state :: map()) ::
              {:ok, state :: map()}
              | {:error, reason :: any()}
              | {:stop, reason :: any(), state :: map()}

  @callback is_authorized(message :: Ockam.Message.t(), state :: map()) ::
              :ok | {:error, reason :: any()}

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

  def get_address(worker, timeout \\ 5000) do
    call(worker, :get_address, timeout)
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
      @default_timeout 30_000

      ## Ignore match errors in handle_info when checking a result of handle_message
      ## handle_message definition may not return {:error, ...} and it shouldn't fail because of that
      @dialyzer {:no_match, handle_info: 2, handle_continue: 2}

      def create(options \\ [], timeout \\ @default_timeout) when is_list(options) do
        Ockam.Worker.create(__MODULE__, options, timeout)
      end

      def start_link(options) when is_list(options) do
        Ockam.Worker.start_link(__MODULE__, options)
      end

      def start_link(address, options) do
        Ockam.Worker.start_link(__MODULE__, address, options)
      end

      def register_extra_addresses(addresses, state) do
        Ockam.Worker.register_extra_addresses(__MODULE__, addresses, state)
      end

      def register_random_extra_address(state) do
        Ockam.Worker.register_random_extra_address(__MODULE__, state)
      end

      @doc false
      @impl true
      def init(options) do
        Ockam.Worker.init(options)
      end

      @doc false
      @impl true
      def handle_info(%Ockam.Message{} = message, state) do
        Ockam.Worker.handle_message(__MODULE__, message, state)
      end

      @doc false
      @impl true
      def handle_continue(:post_init, options) do
        Ockam.Worker.handle_post_init(__MODULE__, options)
      end

      @impl true
      def handle_call(:get_address, _from, %{address: address} = state) do
        {:reply, address, state}
      end

      @doc false
      def setup(_options, state), do: {:ok, state}

      @doc false
      def address_prefix(_options), do: ""

      @doc false
      def is_authorized(message, state) do
        Ockam.Worker.Authorization.with_state_config(message, state)
      end

      defoverridable setup: 2, address_prefix: 1, is_authorized: 2
    end
  end

  ## Functions used from __using__ macro
  ## Moved here for better debugging and to keep the __using__ block short

  def create(module, options, timeout) when is_list(options) do
    case Node.start_supervised(module, options) do
      {:ok, pid, worker} ->
        ## TODO: a better way to handle failing start
        try do
          :sys.get_state(pid, timeout)
          {:ok, worker}
        catch
          _type, err ->
            {:error, {:worker_init, worker, err}}
        end

      error ->
        error
    end
  end

  def start_link(module, options) when is_list(options) do
    address_prefix = Keyword.get(options, :address_prefix, module.address_prefix(options))

    ## Make sure there is no `nil` address in there
    ## TODO: validate address format
    ## TODO: better way to set address than `put_new_lazy`
    options =
      case Keyword.fetch(options, :address) do
        {:ok, nil} -> Keyword.delete(options, :address)
        {:ok, _address} -> options
        :error -> options
      end

    options =
      Keyword.put_new_lazy(options, :address, fn ->
        Node.get_random_unregistered_address(address_prefix)
      end)

    with {:ok, address} <- Keyword.fetch(options, :address),
         {:ok, pid} <- start_link(module, address, options) do
      {:ok, pid, address}
    else
      :error ->
        {:error, {:option_is_nil, :address}}
    end
  end

  def start_link(module, address, options) when is_list(options) and address != nil do
    GenServer.start_link(module, options, name: {:via, Node.process_registry(), address})
  end

  def init(options) do
    {:ok, options, {:continue, :post_init}}
  end

  def handle_post_init(module, options) do
    Node.set_address_module(Keyword.fetch!(options, :address), module)

    return_value =
      with_init_metric(module, options, fn ->
        with {:ok, address} <- Keyword.fetch(options, :address),
             authorization when is_list(authorization) <-
               Keyword.get(options, :authorization, [:to_my_address]) do
          base_state = %{
            address: address,
            all_addresses: [address],
            module: module,
            last_message_ts: nil,
            authorization: Authorization.expand_config(authorization)
          }

          with {:ok, state} <-
                 register_extra_addresses(
                   module,
                   Keyword.get(options, :extra_addresses, []),
                   base_state
                 ) do
            module.setup(options, state)
          end
        else
          :error ->
            {:error, {:option_is_nil, :address}}

          false ->
            {:error, {:option_invalid, :authorization, :not_a_list}}
        end
      end)

    case return_value do
      {:ok, state} ->
        {:noreply, state}

      {:error, reason} ->
        {:stop, reason, {:post_init, options}}
    end
  end

  def handle_message(module, message, state) do
    return_value =
      with_handle_message_metric(module, message, state, fn ->
        with :ok <- module.is_authorized(message, state) do
          module.handle_message(message, state)
        end
      end)

    last_message_ts = System.os_time(:millisecond)

    case return_value do
      {:ok, returned_state} ->
        {:noreply, Map.put(returned_state, :last_message_ts, last_message_ts)}

      {:stop, reason, returned_state} ->
        {:stop, reason, returned_state}

      {:error, reason} ->
        Logger.warn("Worker #{module} handle_message failed. Reason: #{inspect(reason)}")
        {:noreply, Map.put(state, :last_message_ts, last_message_ts)}
    end
  end

  ## Extra address registration

  def register_extra_addresses(module, extra_addresses, state) do
    result =
      Enum.reduce(extra_addresses, :ok, fn
        extra_address, :ok ->
          case Ockam.Node.register_address(extra_address, module) do
            :ok -> :ok
            {:error, reason} -> {:error, {:cannot_register_address, extra_address, reason}}
          end

        _address, error ->
          error
      end)

    case result do
      :ok ->
        {:ok,
         Map.update(state, :all_addresses, extra_addresses, fn all_addresses ->
           extra_addresses ++ all_addresses
         end)}

      error ->
        error
    end
  end

  def register_random_extra_address(module, state) do
    address = Ockam.Node.get_random_unregistered_address()

    case register_extra_addresses(module, [address], state) do
      {:ok, state} -> {:ok, address, state}
      {:error, {:already_registered, _pid}} -> register_random_extra_address(module, state)
      {:error, reason} -> {:error, reason}
    end
  end

  ## Metrics functions

  def with_handle_message_metric(module, message, state, fun) do
    ## TODO: improve metadata
    metadata = %{message: message, address: Map.get(state, :address), module: module}

    start_time = emit_handle_message_start(metadata)

    return_value = fun.()

    emit_handle_message_stop(metadata, start_time, return_value)
    return_value
  end

  defp emit_handle_message_start(metadata) do
    start_time =
      Telemetry.emit_start_event([Map.get(metadata, :module), :handle_message], metadata: metadata)

    Telemetry.emit_event([Ockam.Worker, :handle_message, :start],
      metadata: metadata,
      measurements: %{system_time: System.system_time()}
    )

    start_time
  end

  defp emit_handle_message_stop(metadata, start_time, return_value) do
    result =
      case return_value do
        {:ok, _result} -> :ok
        {:stop, _reason, _state} -> :stop
        {:error, _reason} -> :error
      end

    metadata = Map.merge(metadata, %{result: result, return_value: return_value})

    Telemetry.emit_stop_event([Map.get(metadata, :module), :handle_message], start_time,
      metadata: metadata
    )

    Telemetry.emit_stop_event([Ockam.Worker, :handle_message], start_time, metadata: metadata)
  end

  def with_init_metric(module, options, fun) do
    metadata = %{
      address: Keyword.get(options, :address),
      options: options,
      module: module
    }

    start_time = emit_init_start(metadata)
    return_value = fun.()
    emit_init_stop(metadata, start_time, return_value)
    return_value
  end

  defp emit_init_start(metadata) do
    start_time =
      Telemetry.emit_start_event([Map.get(metadata, :module), :init], metadata: metadata)

    Telemetry.emit_event([Ockam.Worker, :init, :start],
      metadata: metadata,
      measurements: %{system_time: System.system_time()}
    )

    start_time
  end

  defp emit_init_stop(metadata, start_time, return_value) do
    result =
      case return_value do
        {:ok, _state} -> :ok
        {:error, _reason} -> :error
      end

    metadata = Map.merge(metadata, %{result: result, return_value: return_value})
    Telemetry.emit_stop_event([Map.get(metadata, :module), :init], start_time, metadata: metadata)
    Telemetry.emit_stop_event([Ockam.Worker, :init], start_time, metadata: metadata)
  end
end
