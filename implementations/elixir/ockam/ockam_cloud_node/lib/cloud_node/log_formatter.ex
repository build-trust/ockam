defmodule Ockam.CloudNode.LogFormatter do
  @moduledoc """
  Formats lines using Logger.Formatter, but replaces line breaks in log messages with `\n`

  Config:
  `:format_string` in logger config - pattern to format log with

  Usage:
  config :logger, :console,
    format: {Ockam.CloudNode.LogFormatter, :format} # Use this formatter
    format_string: "$time $metadata[$level] $message\n" # Pattern to format the log entry

  """
  def format(level, message, timestamp, metadata) do
    pattern =
      Application.get_env(:logger, :console, [])
      |> Keyword.get(:format_string)
      |> Logger.Formatter.compile()

    message_escaped = String.replace(to_string(message), "\n", "\\n")
    Logger.Formatter.format(pattern, level, message_escaped, timestamp, metadata)
  end
end
