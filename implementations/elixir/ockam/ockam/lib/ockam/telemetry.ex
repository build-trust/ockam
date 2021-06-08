if Code.ensure_loaded?(:telemetry) do
  defmodule Ockam.Telemetry do
    @moduledoc """
    Provides functions to emit `:telemetry` events.
    """

    @typedoc "The event name."
    @type event_name :: atom() | [atom(), ...]

    @typedoc "The event measurements."
    @type event_measurements :: map()

    @typedoc "The event metadata."
    @type event_metadata :: map()

    @typedoc "The option values accepted by emit* functions."
    @type option :: {:measurements, event_measurements()} | {:metadata, event_metadata()}

    @typedoc "The options accepted by emit* functions."
    @type options :: [option()]

    @doc """
    Emits a `:telemetry` event.

    The first argument is `event_name` which may be an atom or a list of atoms
    identifying the event. If `event_name` is an atom, the event that is emitted
    has the name [:ockam, event_name]. If `event_name` is list of atoms, the
    event that is emitted has the name `[:ockam] ++ event_name`.

    The second argument is a keyword list of options:

      * `:measurements` - a map of measurements
      * `:metadata` - a map of metadata

    When the event is emitted, the handler functions attached to the event are
    invoked in the emitting process.

    Always returns `:ok`
    """
    @spec emit_event(event_name(), options()) :: :ok

    def emit_event(event_name, options \\ [])

    def emit_event(event_name, options) when is_atom(event_name),
      do: emit_event([event_name], options)

    def emit_event([first | _rest] = event_name, options)
        when is_list(event_name) and is_atom(first) do
      measurements = Keyword.get(options, :measurements, %{})
      metadata = Keyword.get(options, :metadata, %{})

      :ok = :telemetry.execute([:ockam] ++ event_name, measurements, metadata)
    end

    @doc """
    Emits the `start` event.

    The first argument is `event_name` which may be an atom or a list of atoms
    identifying the event. If `event_name` is an atom, the event that is emitted
    has the name [:ockam, event_name, :start]. If `event_name` is list of atoms,
    the event that is emitted has the name `[:ockam] ++ event_name ++ [:start]`.

    The second argument is a keyword list of options:

      * `:measurements` - a map of measurements
      * `:metadata` - a map of metadata

    When the event is emitted, the handler functions attached to the event are
    invoked in the emitting process.

    Returns the `start_time` as returned by `System.monotonic_time/0`. This
    value should be later passed back into the `Ockam.Telemetry.emit_stop_event/3`
    or the `Ockam.Telemetry.emit_exception_event/4` functions.
    """
    @spec emit_start_event(event_name(), options()) :: start_time :: integer()

    def emit_start_event(event_name, options \\ [])

    def emit_start_event(event_name, options) when is_atom(event_name),
      do: emit_start_event([event_name], options)

    def emit_start_event([first | _rest] = event_name, options)
        when is_list(event_name) and is_atom(first) do
      start_time = System.monotonic_time()

      metadata = Keyword.get(options, :metadata, %{})
      measurements = Keyword.get(options, :measurements, %{})

      measurements = Map.merge(measurements, %{system_time: System.system_time()})

      event_name = Enum.reverse([:start | Enum.reverse(event_name)])
      :ok = :telemetry.execute([:ockam] ++ event_name, measurements, metadata)

      start_time
    end

    @doc """
    Emits the `stop` event.

    The first argument is `event_name` which may be an atom or a list of atoms
    identifying the event. If `event_name` is an atom, the event that is emitted
    has the name [:ockam, event_name, :stop]. If `event_name` is list of atoms,
    the event that is emitted has the name `[:ockam] ++ event_name ++ [:stop]`.

    The second argument is `start_time` that was returned by calling
    `Ockam.Telemetry.emit_start_event/2`. This function will add a `duration`
    measurement to the event by calculating `start_time - end_time`.

    The third argument is a keyword list of options:

      * `:measurements` - a map of measurements
      * `:metadata` - a map of metadata

    When the event is emitted, the handler functions attached to the event are
    invoked in the emitting process.

    Always returns `:ok`.
    """
    @spec emit_stop_event(event_name(), start_time :: integer(), options()) :: :ok

    def emit_stop_event(event_name, start_time, options \\ [])

    def emit_stop_event(event_name, start_time, options)
        when is_atom(event_name) and is_integer(start_time),
        do: emit_stop_event([event_name], start_time, options)

    def emit_stop_event([first | _rest] = event_name, start_time, options)
        when is_list(event_name) and is_atom(first) and is_integer(start_time) do
      measurements = Keyword.get(options, :measurements, %{})
      metadata = Keyword.get(options, :metadata, %{})

      end_time = System.monotonic_time()
      measurements = Map.merge(measurements, %{duration: end_time - start_time})

      event_name = Enum.reverse([:stop | Enum.reverse(event_name)])
      :ok = :telemetry.execute([:ockam] ++ event_name, measurements, metadata)
    end

    @doc """
    Emits the `exception` event.

    The first argument is `event_name` which may be an atom or a list of atoms
    identifying the event. If `event_name` is an atom, the event that is emitted
    has the name [:ockam, event_name, :exception]. If `event_name` is list of
    atoms, the event that is emitted has the name
    `[:ockam] ++ event_name ++ [:exception]`.

    The second argument is `start_time` that was returned by calling
    `Ockam.Telemetry.emit_start_event/2`. This function will add a `duration`
    measurement to the event by calculating `start_time - end_time`.

    The third argument is the kind of exception.

    The fourth argument is the reason for the exception.

    The fifth argument is the stacktrace of the exception.

    The sixth argument is a keyword list of options:

      * `:measurements` - a map of measurements
      * `:metadata` - a map of metadata

    When the event is emitted, the handler functions attached to the event are
    invoked in the emitting process.

    Always returns `:ok`.
    """
    @spec emit_exception_event(
            event_name :: event_name(),
            start_time :: integer(),
            exception ::
              {kind :: Exception.kind(), reason :: any(), stacktrace :: Exception.stacktrace()}
              | map(),
            options :: options()
          ) :: :ok

    def emit_exception_event(event_name, start_time, exception, options \\ [])

    def emit_exception_event(event_name, start_time, exception, options)
        when is_atom(event_name) and is_integer(start_time),
        do: emit_exception_event([event_name], start_time, exception, options)

    def emit_exception_event(event_name, start_time, %{reason: reason} = exception, options) do
      stacktrace = exception |> Map.get(:metadata, %{}) |> Map.get(:stacktrace, [])
      emit_exception_event(event_name, start_time, {:error, reason, stacktrace}, options)
    end

    def emit_exception_event(
          [first | _rest] = event_name,
          start_time,
          {kind, reason, stacktrace},
          options
        )
        when is_list(event_name) and is_atom(first) and is_integer(start_time) do
      measurements = Keyword.get(options, :measurements, %{})
      metadata = Keyword.get(options, :metadata, %{})
      metadata = Map.merge(metadata, %{kind: kind, reason: reason, stacktrace: stacktrace})

      end_time = System.monotonic_time()
      measurements = Map.merge(measurements, %{duration: end_time - start_time})

      event_name = Enum.reverse([:exception | Enum.reverse(event_name)])
      :ok = :telemetry.execute([:ockam] ++ event_name, measurements, metadata)
    end
  end
else
  defmodule Ockam.Telemetry do
    @moduledoc false

    @doc false
    def emit_event(_event_name, _options \\ []), do: :ok

    @doc false
    def emit_start_event(_event_name, _options \\ []), do: 0

    @doc false
    def emit_stop_event(_event_name, _start_time, _options \\ []), do: :ok

    @doc false
    def emit_exception_event(_event_name, _start_time, _exception, _options \\ []), do: :ok
  end
end
