defmodule Ockam.Telemetry.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Telemetry
  alias Ockam.Telemetry

  describe "emit_event/2" do
    test "invokes handler" do
      {function_name, _} = __ENV__.function
      tester_pid = self()

      handler = fn event_name, measurements, metadata, config ->
        send(tester_pid, {event_name, measurements, metadata, config, self()})
      end

      :telemetry.attach(function_name, [:ockam, function_name], handler, nil)

      :ok = Telemetry.emit_event(function_name)

      assert_receive {[:ockam, function_name], measurements, metadata, nil, handler_pid}
      assert handler_pid === tester_pid
      assert %{} === measurements
      assert %{} === metadata

      :telemetry.detach(function_name)
    end

    test "invoked handler receives measurements and metadata" do
      {function_name, _} = __ENV__.function
      tester_pid = self()

      handler = fn event_name, measurements, metadata, config ->
        send(tester_pid, {event_name, measurements, metadata, config, self()})
      end

      :telemetry.attach(function_name, [:ockam, function_name], handler, nil)

      :ok =
        Telemetry.emit_event(function_name, measurements: %{a: 100}, metadata: %{name: "TEST"})

      assert_receive {[:ockam, function_name], measurements, metadata, nil, handler_pid}
      assert handler_pid === tester_pid
      assert %{a: 100} = measurements
      assert %{name: "TEST"} = metadata

      :telemetry.detach(function_name)
    end
  end

  describe "emit_start_event/2" do
    test "invokes handler" do
      {function_name, _} = __ENV__.function
      tester_pid = self()

      handler = fn event_name, measurements, metadata, config ->
        send(tester_pid, {event_name, measurements, metadata, config})
      end

      :telemetry.attach(function_name, [:ockam, function_name, :start], handler, nil)

      start_time = Telemetry.emit_start_event(function_name)

      assert is_integer(start_time)
      assert_receive {[:ockam, function_name, :start], measurements, metadata, nil}
      assert %{system_time: _system_time} = measurements
      assert %{} === metadata

      :telemetry.detach(function_name)
    end

    test "invoked handler receives extra measurements and metadata" do
      {function_name, _} = __ENV__.function
      tester_pid = self()

      handler = fn event_name, measurements, metadata, config ->
        send(tester_pid, {event_name, measurements, metadata, config})
      end

      :telemetry.attach(function_name, [:ockam, function_name, :start], handler, nil)

      start_time = Telemetry.emit_start_event(function_name)

      assert is_integer(start_time)
      assert_receive {[:ockam, function_name, :start], measurements, metadata, nil}
      assert %{system_time: _system_time} = measurements
      assert %{} === metadata

      :telemetry.detach(function_name)
    end
  end

  describe "emit_stop_event/2" do
    test "invokes handler" do
      {function_name, _} = __ENV__.function
      tester_pid = self()

      handler = fn event_name, measurements, metadata, config ->
        send(tester_pid, {event_name, measurements, metadata, config})
      end

      :telemetry.attach(function_name, [:ockam, function_name, :stop], handler, nil)

      start_time = Telemetry.emit_start_event(function_name)
      assert is_integer(start_time)

      :ok = Telemetry.emit_stop_event(function_name, start_time)

      assert_receive {[:ockam, function_name, :stop], measurements, metadata, nil}
      assert %{duration: _duration} = measurements
      assert %{} === metadata

      :telemetry.detach(function_name)
    end
  end

  describe "emit_exception_event/2" do
    test "invokes handler" do
      {function_name, _} = __ENV__.function
      tester_pid = self()

      handler = fn event_name, measurements, metadata, config ->
        send(tester_pid, {event_name, measurements, metadata, config})
      end

      :telemetry.attach(function_name, [:ockam, function_name, :exception], handler, nil)

      start_time = Telemetry.emit_start_event(function_name)
      assert is_integer(start_time)

      stacktrace =
        try do
          raise "err"
        rescue
          error ->
            trace = __STACKTRACE__

            :ok =
              Telemetry.emit_exception_event(function_name, start_time, {:error, error, trace})

            trace
        end

      assert_receive {[:ockam, function_name, :exception], measurements, metadata, nil}

      assert %{duration: _duration} = measurements

      assert %{kind: :error, reason: %RuntimeError{message: "err"}} = metadata

      assert stacktrace === metadata.stacktrace

      :telemetry.detach(function_name)
    end
  end
end
