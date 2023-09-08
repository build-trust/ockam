defmodule EarmarkParser.Context do

  @moduledoc false
  use EarmarkParser.Types

  @type t :: %__MODULE__{
          options: EarmarkParser.Options.t(),
          links: map(),
          footnotes: map(),
          value: String.t() | [String.t()]
        }

  defstruct options: %EarmarkParser.Options{},
            links: Map.new(),
            rules: nil,
            footnotes: Map.new(),
            value: []

  ##############################################################################
  # Handle adding option specific rules and processors                         #
  ##############################################################################

  @doc false
  # Convenience method to append to the value list
  # def append(%__MODULE__{value: value} = ctx, prep), do: %{ctx | value: [value | prep]}

  def modify_value(%__MODULE__{value: value}=context, fun) do
    # IO.inspect(value, label: ">>>modify_value")
    nv = fun.(value) #|> IO.inspect(label: "<<<modify_value")
    unless is_list(nv), do: raise "Not a list!!!\n#{inspect nv}"
    %{context | value: nv}
  end

  @doc false
  # Convenience method to prepend to the value list
  def prepend(context, ast, messages \\ [])
  def prepend(%__MODULE__{} = ctx, prep, messages) do
    options1 = %{ctx.options | messages: Enum.uniq(ctx.options.messages ++ messages)}
    _prepend(%{ctx|options: options1}, prep)
  end

  defp _prepend(ctxt, []), do: ctxt
  defp _prepend(%{value: value}=ctxt, {:comment, _, _, _}=ct), do: %{ctxt|value: [ct|value]}
  defp _prepend(%{value: value}=ctxt, tuple) when is_tuple(tuple) do
    %{ctxt|value: [tuple|value] |> List.flatten}
  end
  defp _prepend(%{value: value}=ctxt, list) when is_list(list), do: %{ctxt|value: List.flatten(list ++ value)}

  @doc """
  Convenience method to prepend to the value list
  """
  def set_value(%__MODULE__{} = ctx, value) do
    # TODO: Remove me
    unless is_list(value), do: raise "Not a list!!!\n#{inspect value}"
    %{ctx | value: value}
  end

  def clear_value(%__MODULE__{} = ctx), do: %{ctx | value: []}

  # this is called by the command line processor to update
  # the inline-specific rules in light of any options
  def update_context(context = %EarmarkParser.Context{options: options}) do
    %{context | rules: rules_for(options)}
  end

  #                 ( "[" .*? "]"n or anything w/o {"[", "]"}* or "]" ) *
  @link_text ~S{(?:\[[^]]*\]|[^][]|\])*}
  # "
  # @href ~S{\s*<?(.*?)>?(?:\s+['"](.*?)['"])?\s*}


  defp basic_rules do
    [
      br: ~r<^ {2,}\n(?!\s*$)>,
      text: ~r<^[\s\S]+?(?=[\\<!\[_*`]| {2,}\n|$)>,
    ]
  end

  defp rules_for(options) do
    rule_updates =
      if options.gfm do
        rules = [
          escape: ~r{^\\([\\`*\{\}\[\]()\#+\-.!_>~|])},
          url: ~r{^(https?:\/\/[^\s<]+[^<.,:;\"\')\]\s])},
          strikethrough: ~r{^~~(?=\S)([\s\S]*?\S)~~},
          text: ~r{^[\s\S]+?(?=[\\<!\[_*`~]|https?://| \{2,\}\n|$)}
        ]

        if options.breaks do
          break_updates = [
            br: ~r{^ *\n(?!\s*$)},
            text: ~r{^[\s\S]+?(?=[\\<!\[_*`~]|https?://| *\n|$)}
          ]

          Keyword.merge(rules, break_updates)
        else
          rules
        end
      else
        []
      end

    footnote = if options.footnotes, do: ~r{^\[\^(#{@link_text})\]}, else: ~r{\z\A}
    rule_updates = Keyword.merge(rule_updates, footnote: footnote)

    Keyword.merge(basic_rules(), rule_updates)
    |> Enum.into(%{})
  end

end

# SPDX-License-Identifier: Apache-2.0
