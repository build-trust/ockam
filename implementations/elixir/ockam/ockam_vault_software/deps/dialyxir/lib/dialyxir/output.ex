defmodule Dialyxir.Output do
  alias IO.ANSI

  def color(text, color) when is_binary(text) do
    ANSI.format([color, text])
  end

  def info(""), do: :ok
  def info(text), do: Mix.shell().info(text)

  def error(""), do: :ok
  def error(text), do: Mix.shell().error(text)
end
