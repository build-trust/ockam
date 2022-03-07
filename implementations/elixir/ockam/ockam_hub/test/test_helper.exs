Application.ensure_all_started(:ockam)

defmodule Test.Utils do
  @moduledoc """
  Util functions for testing
  """

  require IEx.Helpers

  def pid_to_string() do
    pid_to_string(self())
  end

  def pid_to_string(process_id) when is_pid(process_id) do
    pid_to_string("#{inspect(process_id)}")
  end

  def pid_to_string(process_id) when is_binary(process_id) do
    pattern = ~r/#PID<([0-9]+.[0-9]+.[0-9]+)>/
    [^process_id, str] = Regex.run(pattern, process_id)
    str
  end

  def string_to_pid(process_id) when is_binary(process_id) do
    IEx.Helpers.pid(process_id)
  end
end

ExUnit.start(capture_log: true, trace: true)
