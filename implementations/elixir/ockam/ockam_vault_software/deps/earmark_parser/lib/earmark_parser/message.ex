defmodule EarmarkParser.Message do

  @moduledoc false

  alias EarmarkParser.Context
  alias EarmarkParser.Options

  @type message_type :: :error | :warning
  @type t :: {message_type, number, binary}
  @type ts:: list(t)
  @type container_type :: Options.t | Context.t

  def add_messages(container, messages), do:
    Enum.reduce(messages, container, &(add_message(&2, &1)))

  def add_message(container, message)
  def add_message(options = %Options{}, message) do
    %{options | messages: [message | options.messages]}
  end
  def add_message(context = %Context{}, message) do
    %{context | options: %{context.options | messages: [message | get_messages(context)]}}
  end
  
  def add_messages_from(context, message_container) do
    add_messages(context, message_container.options.messages)
  end

  def get_messages(container)
  def get_messages(%Context{options: %{messages: messages}}), do: messages
  def get_messages(%Options{messages: messages}),             do: messages

  def set_messages(container, messages)
  def set_messages(c = %Context{}, messages), do: put_in(c.options.messages, messages)

  def emit_messages(messages, %Options{file: file}) do
    messages
    |> Enum.each(&(emit_message(file, &1)))
  end

  @doc """
  For final output
  """
  def sort_messages(container) do
    container
    |> get_messages()
    |> Enum.sort( fn ({_,l,_}, {_,r,_}) -> r >= l end )
  end

  defp emit_message(filename, msg), do:
    IO.puts(:stderr, format_message(filename, msg))

  defp format_message filename, {type, line, text} do
    "#{filename}:#{line}: #{type}: #{text}"
  end

end

# SPDX-License-Identifier: Apache-2.0
