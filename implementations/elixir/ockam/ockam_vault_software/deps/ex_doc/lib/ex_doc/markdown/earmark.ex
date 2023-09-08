defmodule ExDoc.Markdown.Earmark do
  @moduledoc """
  ExDoc extension for the EarmarkParser Markdown parser.
  """

  @behaviour ExDoc.Markdown

  @impl true
  def available? do
    match?({:ok, _}, Application.ensure_all_started(:earmark_parser)) and
      Code.ensure_loaded?(EarmarkParser)
  end

  @doc """
  Generate HTML AST.

  ## Options

    * `:gfm` - (boolean) turns on Github Flavored Markdown extensions. Defaults to `true`.

    * `:breaks` - (boolean) only applicable if `gfm` is enabled. Makes all line
      breaks significant (so every line in the input is a new line in the output).

    * `:smartypants` - (boolean) turns on smartypants processing, so quotes become curly,
      two or three hyphens become en and em dashes, and so on. Defaults to `false`.

  """
  @impl true
  def to_ast(text, opts) do
    options = [
      gfm: true,
      line: 1,
      file: "nofile",
      breaks: false,
      smartypants: false,
      pure_links: true
    ]

    options = Keyword.merge(options, opts)

    case EarmarkParser.as_ast(text, options) do
      {:ok, ast, messages} ->
        print_messages(messages, options)
        fixup(ast)

      {:error, ast, messages} ->
        print_messages(messages, options)
        fixup(ast)
    end
  end

  defp print_messages(messages, options) do
    for {severity, line, message} <- messages do
      file = options[:file]
      IO.warn("#{inspect(__MODULE__)} (#{severity}) #{file}:#{line} #{message}", [])
    end
  end

  defp fixup(list) when is_list(list) do
    fixup_list(list, [])
  end

  defp fixup(binary) when is_binary(binary) do
    binary
  end

  defp fixup({tag, attrs, ast}) do
    fixup({tag, attrs, ast, %{}})
  end

  defp fixup({tag, attrs, ast, meta}) when is_binary(tag) and is_list(attrs) and is_map(meta) do
    {fixup_tag(tag), Enum.map(attrs, &fixup_attr/1), fixup(ast), meta}
  end

  defp fixup({:comment, _, _, _}) do
    []
  end

  defp fixup_list([head | tail], acc) do
    fixed = fixup(head)

    if fixed == [] do
      fixup_list(tail, acc)
    else
      fixup_list(tail, [fixed | acc])
    end
  end

  defp fixup_list([], acc) do
    Enum.reverse(acc)
  end

  defp fixup_tag(tag) do
    String.to_atom(tag)
  end

  defp fixup_attr({name, value}) do
    {String.to_atom(name), value}
  end
end
